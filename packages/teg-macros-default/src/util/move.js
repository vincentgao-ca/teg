import {
  ComponentTypeEnum,
} from '@tegapp/core'
import getMoveComponents from './getMoveComponents'

const { TOOLHEAD } = ComponentTypeEnum

const move = ({
  axes,
  sync,
  relativeMovement,
  allowExtruderAxes,
  machineConfig,
  feedrate,
}) => {
  if (feedrate != null && feedrate < 0) {
    throw new Error(`feedrate must be greater then zero if set. Got: ${feedrate}`)
  }

  const g1Args = {}
  const feedrates = []

  getMoveComponents({
    axes,
    allowExtruderAxes,
    machineConfig,
  }).forEach(({ component, address, value }) => {
    if (typeof value !== 'number' || Number.isNaN(value)) {
      throw new Error(`${address}: ${value} is not a number`)
    }

    const isToolhead = component.type === TOOLHEAD

    // TODO: does this work with multi-extruder printers?
    g1Args[(isToolhead ? 'e' : address)] = value

    feedrates.push(component.model.get('feedrate'))
  })

  const f = (feedrate || Math.min.apply(null, feedrates)) * 60

  const commands = [
    relativeMovement ? 'G91' : 'G90',
    { g1: { f } },
    { g1: { ...g1Args, f } },
    'G90',
    /*
    * Synchronize the end of the task with M400 by waiting until all
    * scheduled movements in the task are finished.
    */
    ...(sync === true ? ['M400'] : []),
  ]

  return { commands }
}

export default move
