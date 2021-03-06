// import { StatsReport } from "apollo-engine-reporting-protobuf"
import getComponents from '../../config/selectors/getComponents'
import { TOOLHEAD } from '../../config/types/components/ComponentTypeEnum'
import { NullSchemaForm } from '../../pluginManager/types/SchemaForm'
// import getAxePositions from '../selectors/getAxePositions'

const ComponentResolvers = {
  Component: {
    name: (source, args, { store }) => {
      const state = store.getState()
      const machineConfig = state.config.machines.get(source.machineID)

      const component = getComponents(machineConfig).get(source.id)

      return component.model.get('name')
    },
    configForm: (source, args, { store }) => {
      const state = store.getState()
      const machineConfig = state.config.machines.get(source.machineID)

      const component = getComponents(machineConfig).get(source.id)

      if (component == null) {
        throw new Error(`Cannot find config for component: ${source.id}`)
      }

      const { id, model, modelVersion } = component

      const schemaForm = state.schemaForms.getIn(
        ['components', source.type],
        NullSchemaForm,
      )

      return {
        id,
        model,
        modelVersion,
        schemaForm,
      }
    },
    toolhead: (source, args, { store }) => {
      // TODO: isn't this too 3D printer specific?
      const state = store.getState()
      const machineConfig = state.config.machines.get(source.machineID)

      const component = getComponents(machineConfig).get(source.id)

      if (component == null) {
        throw new Error(`Cannot find config for component: ${source.id}`)
      }

      if (component.type === TOOLHEAD) {
        return component
      }

      return null
    },
  },
}

export default ComponentResolvers
