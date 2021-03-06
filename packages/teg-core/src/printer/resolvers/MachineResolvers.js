import { request } from 'graphql-request'

import busyMachines, { BUSY_WITH_JOB } from '../../jobQueue/selectors/busyMachines'
// import getComponents from '../../config/selectors/getComponents'
// import getComponentsState from '../selectors/getComponentsState'
import getPluginModels from '../../config/selectors/getPluginModels'
// import ComponentTypeEnum from '../../config/types/components/ComponentTypeEnum'
import getMachineConfigForm from '../../config/selectors/getMachineConfigForm'
import {
  READY,
} from '../types/statusEnum'

const MachineResolvers = {
  Machine: {
    name: (source, args, { store }) => {
      const state = store.getState()
      const machineConfig = state.config.machines.get(source.id)

      return getPluginModels(machineConfig).getIn(['@tegapp/core', 'name'])
    },

    swapXAndYOrientation: (source, args, { store }) => {
      const state = store.getState()
      const machineConfig = state.config.machines.get(source.id)

      return getPluginModels(machineConfig).getIn(['@tegapp/core', 'swapXAndYOrientation'], false)
    },

    fixedListComponentTypes: (source, args, { store }) => {
      const state = store.getState()

      return state.fixedListComponentTypes
    },

    // targetTemperaturesCountdown: source => (
    //   getComponentsState(source).targetTemperaturesCountdown
    // ),

    // activeExtruderID: source => getComponentsState(source).activeExtruderID,

    enabledMacros: (source, args, { store }) => {
      const state = store.getState()

      return state.macros.get(source.machineID)
    },

    availablePackages: (source, args, { store }) => {
      const state = store.getState()
      const machineConfig = state.config.machines.get(source.id)

      const { availablePlugins } = state.pluginManager

      const installedPackages = machineConfig.plugins.map(p => p.package)

      return Object.keys(availablePlugins).filter(packageName => (
        installedPackages.includes(packageName) === false
      ))
    },

    configForm: (source, args, { store }) => {
      const state = store.getState()

      // TODO: multimachine config forms
      const {
        model,
        modelVersion,
        schemaForm,
      } = getMachineConfigForm(state)

      return {
        id: source.id,
        model,
        modelVersion,
        schemaForm,
      }
    },

    components: (source, args) => {
      // TODO: id-based lookup

      const id = args.componentID
      if (id != null) {
        const component = source.components.find(c => c.id === id.replace('component-', ''))
        if (component == null) {
          throw new Error(`Component ID: ${id} does not exist`)
        }
        return [component]
      }
      return source.components.toList()
    },

    plugins: (source, args, { store }) => {
      const state = store.getState()
      const { plugins } = state.config.machines.get(source.id)

      if (args.package != null) {
        const plugin = plugins.find(p => p.package === args.package)
        if (plugin == null) {
          throw new Error(`Plugin package: ${args.package} does not exist`)
        }
        return [plugin]
      }

      return plugins
    },

    // status: async (source, args, { store }) => {
    //   const state = store.getState()

    //   const { status } = source
    //   if (
    //     busyMachines(state.jobQueue)[source.id] === BUSY_WITH_JOB
    //     && status === READY
    //   ) {
    //     return 'PRINTING'
    //   }

    //   return status.substring(status.lastIndexOf('/') + 1)
    // },

    status: async (source) => {
      const query = `
        query(
          $machineID: ID!
        ) {
          machines(id: $machineID) {
            status
          }
        }
      `

      const variables = {
        machineID: source.id,
      }

      const data = await request('http://127.0.0.1:33005/graphql', query, variables)

      return data.machines[0].status
    },

    // logEntries: (source, args) => {
    //   let entries = source.log.get('logEntries')
    //   if (args.level != null) {
    //     entries = entries.filter(log => log.level === args.level)
    //   }
    //   if (args.sources != null) {
    //     entries = entries.filter(log => args.sources.includes(log.source))
    //   }
    //   if (args.limit != null) {
    //     entries = entries.slice(0, args.limit)
    //   }
    //   return entries.toArray()
    // },

    gcodeHistory: (source, args, { store }) => {
      const state = store.getState()

      let { historyEntries } = state.gcodeHistory.get(source.id)

      if (args.limit != null) {
        historyEntries = historyEntries.slice(-args.limit)
      }

      return historyEntries.toArray()
    },
    // movementHistory: (source, args, { store }) => {
    //   const state = store.getState()
    //   return getComponentsState(state).movementHistory
    //     .toArray()
    // },
  },
}

export default MachineResolvers
