import { Record, Map } from 'immutable'
import libUUID from 'uuid'

import CrashReportConfig from './CrashReportConfig'
import MaterialConfig from './MaterialConfig'
import LogConfig from './LogConfig'

export const HostConfigRecordFactory = Record({
  id: null,
  localID: null,
  modelVersion: 0,
  configDirectory: '/etc/teg',
  name: null,
  crashReports: CrashReportConfig(),
  model: Map(),
  log: LogConfig(),
})

const MAX_U32 = (2 ** 32) - 1

const HostConfig = ({
  id = libUUID.v4(),
  localID = Math.floor(Math.random() * MAX_U32),
  modelVersion = 0,
  crashReports = {},
  materials = [],
  log = {},
  ...props
} = {}) => (
  HostConfigRecordFactory({
    ...props,
    id,
    localID,
    modelVersion,
    crashReports: CrashReportConfig(crashReports),
    materials: materials.map(MaterialConfig),
    log: LogConfig(log),
    model: Map(props.model),
  })
)

export default HostConfig
