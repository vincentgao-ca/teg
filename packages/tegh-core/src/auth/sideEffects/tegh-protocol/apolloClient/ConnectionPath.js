import DatConnection from '../connection/dat/DatConnection'
import WebSocketConnection from '../connection/webSocket/WebSocketConnection'
import UpgradeToWebRTC from '../connection/webRTC/UpgradeToWebRTC'

import InitiatorHandshake from '../handshake/InitiatorHandshake'

const SIGNALLING_WEBSOCKET_URL = 'ws://v0-60-signalling.tegh.io'

const signallingWebsocket = (() => {
  let ws = null

  return () => {
    // eslint-disable-next-line no-undef
    if (ws == null) ws = new WebSocket(SIGNALLING_WEBSOCKET_URL)
    return ws
  }
})()

const ConnectionPath = ({
  identityKeys,
  peerIdentityPublicKey,
}) => {
  let initialConnection

  // TODO: getDatIDFromPublicKey
  const peerDatID = peerIdentityPublicKey

  // eslint-disable-next-line no-undef
  const { peers } = experimental || {}

  if (peers != null) {
    initialConnection = DatConnection({
      peers,
      peer: peers.get(peerDatID),
    })
  } else {
    initialConnection = WebSocketConnection({
      websocket: signallingWebsocket(),
      peerIdentityPublicKey,
    })
  }

  const connectionPath = [
    // connect to Dat Peer or tunnel through a WebSocket to the peer
    initialConnection,
    // Establish a secure connection
    InitiatorHandshake({
      identityKeys,
      peerIdentityPublicKey,
    }),
    // Exchange SDPs over the secure connection and switch to WebRTC
    UpgradeToWebRTC(),
  ]
  return connectionPath
}

export default ConnectionPath