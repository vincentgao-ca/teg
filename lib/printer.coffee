EventEmitter = require('events').EventEmitter
PrintDriver = require("../lib/print_driver.coffee")
PrintJob = null
require 'sugar'
chai = require("chai")
chai.should()

module.exports = class Printer extends EventEmitter

  _nextJobId: 0
  _defaultComponents:
    e0: 'heater', b: 'heater', c: 'conveyor', f: 'fan'
  _defaultAttrs:
    heater:
      type: 'heater'
      target_temp: 0
      current_temp: 0
      target_temp_countdown: 0
      flowrate: 40
      blocking: false
    conveyor: { type: 'conveyor', enabled: false }
    fan: { type: 'fan', enabled: false, speed: 255 }

  constructor: (@driver, settings = {}, components, @_PrintJob=PrintJob) ->
    components ?= @_defaultComponents
    @_jobs = []
    # Building the printer data
    @data = {status: 'idle', xy_feedrate: 3000, z_feedrate: 300}
    Object.merge @data, settings
    @data[k] = Object.clone(@_defaultAttrs[v]) for k, v of components
    # Adding the extruders to the axes
    @_axes = ['x','y','z']
    (@_axes.push k if k.startsWith 'e') for k, v of components
    # Adding a status getter
    @__defineGetter__ "status", @_getStatus
    # Updating data on driver change
    @driver.on "change", @_updateData

  _setStatus: (status) ->
    @_updateData status: status

  _getStatus: =>
    @data.status

  addJob: (jobAttrs) ->
    jobAttrs =
      id: @_nextJobId++
      qty: jobAttrs['qty'] || 1
      gcode: jobAttrs['gcode']
      position: @_jobs.length
    job = new @_PrintJob(jobAttrs)
    @_jobs.push job
    @emit "add", "jobs[#{job.id}]", job

  rmJob: (jobAttrs) ->
    job = @_jobs.find (job) -> job.id == jobAttrs.id
    throw "job does not exist" unless job?
    @_jobs.remove(job)
    @emit "remove", "jobs[#{jobAttrs.id}]"

  changeJob: (jobAttrs, whitelistAttrs = true) ->
    if whitelistAttrs
      whitelist = ['id', 'position', 'qty']
      jobAttrs = Object.reject jobAttrs, (k,v) -> whitelist.has(k)
    # Validations
    for k, v of jobAttrs
      continue if @["_validateJob#{k.capitalize()}"](v)
      throw "Invalid #{k}: #{v}"
    job = @_jobs.find( (someJob) -> someJob.id == jobAttrs.id )
    throw "Invalid id: #{jobAttrs.id}" unless job?
    event  = {}
    # Reordering the other jobs if position has changed
    pos = old: job.position, new: (jobAttrs?.position||job.position)
    @_jobs.filter((j) -> j.id != job.id).each (j) ->
      originalPosition = j.position
      j.position += 1 if pos.old > j.position >= pos.new
      j.position -= 1 if pos.old < j.position <= pos.new
      return if j.position == originalPosition
      event["jobs[#{j.id}]"] = position: j.position
    # Saving the data
    delete jobAttrs['id']
    Object.merge job, jobAttrs
    event["jobs[#{job.id}]"] = jobAttrs
    @emit "change", event

  _validateJobId: (val) ->
    val >= 0

  _validateJobQty: (val) ->
    val > 0

  _validateJobPosition: (val) ->
    val >= 0 and val < @_jobs.length

  getJobs: ->
    @_jobs

  estop: ->
    @driver.reset()
    @_setStatus 'estopped'

  # set any number of the following printer attributes:
  # - extruder/bed target_temp
  # - fan enabled
  # - fan speed
  # - conveyor enabled
  set: (diff) ->
    # Fail fast (part 1)
    diff.should.be.a 'object', 'set must be called with an object of changes.'
    diff.should.not.include 'status', 'cannot set status directly.'
    # Setup
    diff = Object.extended(diff)
    temps = diff.map (k, v) -> ( [k, v.target_temp] if v.target_temp ).compact()
    conveyors = diff.filter (k, v) -> v.type == 'conveyor'
    fans = diff.filter (k, v) -> v.type == 'fan'
    # Fail fast (part 2)
    if @status == 'printing'
      temps.should.be.empty     'cannot set temperature while printing.'
      conveyors.should.be.empty 'cannot set conveyor while printing.'
    @_updateData(diff)
    # Send gcodes to the print driver to update the printer
    @driver.sendNow("#{_tempGCode(t[0])} S#{t[1]}") for t in temps
    @_updateConveyor k, @data[k], v for k, v of conveyors
    @_updateFan      k, @data[k], v for k, v of fans

  _tempGCode: (k) ->
    return "M140" if k == 'b'
    return "M104" if k == 'e0'
    return "M104 P#{k[1..]}"

  _updateConveyor: (key, conveyor, diff) ->
    return unless conveyor.enabled or diff.enabled? # (enabled or en. changed)
    @driver.sendNow( if conveyor.enabled then "M240" else "M241" )

  _updateFan: (key, fan, diff) ->
    return unless fan.enabled or diff.enabled? # (enabled or en. changed)
    @driver.sendNow( if fan.enabled then "M106 S#{data.speed}" else "M107" )

  _updateData: (new_data) ->
    # changes = Object.map new_data, @_appendChanges, {}
    changes = {}
    @_appendChanges changes, k, v for k, v of new_data
    # apply the changes and emit a change event
    Object.merge @data, changes, true
    @emit "change", changes

  _appendChanges: (changes, k, v) ->
    if typeof(v) == "object"
      # Fail fast
      for k2, v2 in v
        continue unless typeof(v2) != typeof(@data[k][k2])
        throw "#{k}.#{k2} must be a #{typeof(@data[k][k2])}." if @data[k][k2]?
        throw "#{k}.#{k2} does not exist."
      # Push any modified attributes to the changes
      (changes[k]?={})[k2] = v2 for k2, v2 in v if @data[k][k2] != v2

    # Numbers and Strings
    else if typeof(v2) != typeof(@data[k][k2])
      throw "#{k} must be a #{typeof(@data[k])}." if @data[k]?
      throw "#{k} does not exist."
    else
      changes[k] = v
    return changes

  print: ->
    # Fail fast
    throw "Already printing." if @status == 'printing'
    @_assert_not_idle 'print'
    # Implementation
    job = @_jobs[0]
    @driver.print job
    @_setStatus 'printing'
    @changeJob id: job.id, status: 'printing', false

  move: (axesVals) ->
    # Fail fast
    @_assert_idle 'move'
    err = "move must be called with a object of axes/distance key/values."
    axesVals.should.be.a 'object', err
    axesVals = Object.extended(axesVals)
    axes = Object.keys(axesVals).exclude((k) => @_axes.some(k))
    @_asert_no_bad_axes 'move', axes
    # Adding the axes values
    # gcode = axesVals.reduce ((s, k, v) -> "#{s} #{k.toUpperCase()}#{v}"), 'G1'
    gcode = 'G1'
    gcode += " #{k.replace(/e\d/, 'e').toUpperCase()}#{v}" for k, v of axesVals
    # Calculating and adding the feedrate
    feedrate = @data["#{if axesVals.z? then 'z' else 'xy'}_feedrate"]
    extruders = axesVals.keys().filter (k) -> k.startsWith 'e'
    eFeedrates = extruders.map (k) => @data[k].flowrate
    feedrate = eFeedrates.reduce ( (f1, f2) -> Math.min f1, f2 ), feedrate
    gcode = "G1 F#{feedrate}\n#{gcode} F#{feedrate}"
    if extruders.length > 0
      gcode = "T#{extruders[0].replace 'e', ''}\n#{gcode}"
    # Sending the gcode
    @driver.sendNow gcode

  home: (axes = ['x', 'y', 'z']) ->
    # Fail fast
    @_assert_idle 'home'
    axes.should.be.a 'array', "home must be called with an array of axes."
    @_asert_no_bad_axes 'home', axes.reject((k,v) -> @_axes.has(k))
    # Implementation
    gcode = "G28 " + axes.map( (k) -> "#{k.toUpperCase()}0" )
    @driver.sendNow gcode

  _assert_idle: (method_name) -> 
    @status.should.equal 'idle', "Cannot #{method_name} when #{@status}."

  _asert_no_bad_axes: (methodName, badAxes) ->
    s = badAxes.join ','
    err = "#{methodName} must be called with valid axes. #{s} are invalid."
    badAxes.should.have.length 0, err
