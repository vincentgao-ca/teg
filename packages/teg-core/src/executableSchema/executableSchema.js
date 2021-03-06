import {
  makeExecutableSchema,
  mergeSchemas,
  introspectSchema,
  makeRemoteExecutableSchema,
  transformSchema,
  FilterRootFields,
  FilterTypes,
} from 'graphql-tools'
import { HttpLink } from 'apollo-link-http'
import { setContext } from 'apollo-link-context'

import typeDefs from '@tegapp/schema'

import resolvers from './resolvers'

const executableSchema = async () => {
  const nodeJSSchema = makeExecutableSchema({
    typeDefs,
    resolvers,
    resolverValidationOptions: {
      requireResolversForResolveType: false,
    },
  })

  const rustURI = 'http://localhost:33005/graphql'

  let introspectionResult

  while (introspectionResult == null) {
    try {
      console.error('Connecting to rust server...')
      introspectionResult = await introspectSchema(
        new HttpLink({ uri: rustURI }),
      )
      // console.log({ introspectionResult })
      // console.log({...introspectionResult._mutationType._fields})
      console.error('Connecting to rust server... [DONE]')
    } catch (e) {
      console.error('Unable to connect to rust server. Retrying in 500ms.')
      await new Promise(resolve => setTimeout(resolve, 500))
    }
  }

  // console.log({ introspectionResult })

  const link = setContext((request, previousContext) => ({
    headers: {
      'user-id': previousContext.graphqlContext.user.id,
      'session-id': previousContext.graphqlContext.sessionID,
      'peer-identity-public-key': previousContext.graphqlContext.peerIdentityPublicKey,
    },
  })).concat(new HttpLink({ uri: rustURI }))

  const rustSchema = await makeRemoteExecutableSchema({
    schema: introspectionResult,
    link,
  })

  const ommittedRustFields = [
    'authenticateUser',
  ]

  let ommittedNodeJSFields = [
  ]
  let ommittedNodeJSTypes = [
  ]

  if (process.env.LEGACY_NODE_APIS == null) {
    ommittedNodeJSFields = [
      ...ommittedNodeJSFields,
      'createJob',
      'deleteJob',
      'setJobPosition',
      'jobQueue',
      'execGCodes',
      'spoolJobFile',
    ]
    ommittedNodeJSTypes = [
      ...ommittedNodeJSTypes,
      'CreateJobInput',
      'DeleteJobInput',
      'SetJobPositionInput',
      'ExecGCodesInput',
      'SpoolJobFileInput',
      'JobQueue',
      'Job',
      'JobFile',
      'Task',
    ]
  }

  const transformedRustSchema = transformSchema(rustSchema, [
    new FilterRootFields(
      (_operation, rootField) => ommittedRustFields.includes(rootField) === false,
    ),
  ])

  const transformedNodeJSSchema = transformSchema(nodeJSSchema, [
    new FilterRootFields(
      (_operation, rootField) => ommittedNodeJSFields.includes(rootField) === false,
    ),
    new FilterTypes(
      type => ommittedNodeJSTypes.includes(type.name) === false,
    ),
  ])

  return mergeSchemas({
    schemas: [
      transformedRustSchema,
      transformedNodeJSSchema,
    ],
  })
}

export default executableSchema
