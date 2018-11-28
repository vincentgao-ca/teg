import { Record, Map } from 'immutable'
import uuid from 'uuid/v4'

import { BUILD_PLATFORM } from './ComponentTypeEnum'

export const BuildPlatformConfigFactory = Record({
  id: null,
  modelVersion: 0,
  address: null,
  type: BUILD_PLATFORM,
  name: null,
  heater: false,
  extendedConfig: Map(),
})

const BuildPlatformConfig = ({
  id = uuid(),
  modelVersion = 0,
  ...props
} = {}) => (
  BuildPlatformConfigFactory({
    ...props,
    id,
    modelVersion,
    extendedConfig: Map(props.extendedConfig),
  })
)

export default BuildPlatformConfig