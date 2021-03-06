// import Dat from '@beaker/dat-node'
import wrtc from 'wrtc'

import { execute, subscribe } from 'graphql'
import { SubscriptionServer } from 'subscriptions-transport-ws'
import { getPluginModels } from '@tegapp/core'

import { GraphQLThing } from 'graphql-things'

const webRTCServer = async ({
  schema,
  context,
  identityKeys,
  authenticate,
}) => {
  // console.log("NODE_ENV:", process.env.NODE_ENV)

  // // instantiate the dat node
  // const DAT_URL = (
  //   process.env.DAT_URL
  //   || 'dat://0588d04a52162b001c489a68182daac3de41a18487f88e8a93b6a69fbd38b1ed/'
  // )
  //
  // const dat = Dat.createNode({
  //   path: './.dat-data',
  // })
  // const datPeers = dat.getPeers(DAT_URL)

  console.error('listening for WebRTC connections to', identityKeys.publicKey)

  const thing = GraphQLThing({
    // datPeers,
    identityKeys,
    authenticate,
    wrtc,
    meta: {
      schemaVersion: '0.8',
      schemaExtensions: [],
      get name() {
        const state = context.store.getState()
        return getPluginModels(state.config.printer).getIn(['@tegapp/core', 'name'])
      },
    },
  })

  const options = {
    execute,
    subscribe,
    schema,
    keepAlive: 500,

    // the onOperation function is called for every new operation
    // and we use it to inject context to track the session and
    // user
    onOperation: async (msg, params, socket) => ({
      ...params,
      context: {
        ...context,
        ...socket.authContext,
        sessionID: socket.sessionID,
      },
      formatResponse: (res) => {
        if (res.errors) {
          res.errors.forEach((err, i) => {
            // eslint-disable-next-line no-console
            console.error(`Teg GraphQL Error ${i + 1} / ${res.errors.length} at [${err.path}]: ${err.message}`)
            // eslint-disable-next-line no-console
            console.error(err.stack)
          })
        }
        return res
      },
      formatError: (err) => {
        // eslint-disable-next-line no-console
        console.error(`Teg GraphQL Serious Error: ${err.message}`)
        // eslint-disable-next-line no-console
        console.error(err.stack)
        return err
      },
    }),
  }

  SubscriptionServer.create(options, thing)
}

export default webRTCServer
