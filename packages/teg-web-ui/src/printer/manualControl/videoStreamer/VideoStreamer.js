import React, { useState, useRef, useCallback } from 'react'
import { Link, useHistory } from 'react-router-dom'
import {
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Tooltip,
  Fab,
} from '@material-ui/core'
import { makeStyles } from '@material-ui/core/styles'
import { useAsync } from 'react-async'

import { useApolloClient } from 'react-apollo-hooks'
import gql from 'graphql-tag'

import SimplePeer from 'simple-peer'

import LoadingOverlay from '../../../common/LoadingOverlay'

const createVideoSDPMutation = gql`
  mutation createVideoSDPMutation($offer: RTCSignalInput!) {
    createVideoSDP(offer: $offer) {
      type
      sdp
      candidates {
        candidate
        sdpMLineIndex
      }
    }
  }
`

const useStyles = makeStyles(() => ({
  container: {
    display: 'flex',
    justifyContent: 'center',
    background: 'black',
  },
  video: {
    width: '100%',
    maxHeight: '40vh',
  },
}))

const enhance = Component => (props) => {
  const apollo = useApolloClient()

  const videoEl = useRef(null)
  const [peerError, setPeerError] = useState()

  const loadVideo = useCallback(async () => {
    const mediaConstraints = {
      // offerToReceiveAudio: true,
      offerToReceiveVideo: true,
      offerToReceiveAudio: false,
      // offerToReceiveVideo: false,
    }

    // let streamOut = await navigator.mediaDevices.getUserMedia({
    //   video: true,
    //   audio: true
    // })
    // console.log({ streamOut })

    const p = new SimplePeer({
      // stream: streamOut,
      initiator: true,
      trickle: false,
      offerOptions: mediaConstraints,
      config: {
        iceServers: [
          {
            urls: 'stun:global.stun.twilio.com:3478?transport=udp'
          },
        ],
      }
    })

    p.on('error', err => setPeerError(err))

    const offer = await new Promise(resolve => p.on('signal', resolve))
    // console.log('offer', offer)

    const { data } = await apollo.mutate({
      mutation: createVideoSDPMutation,
      variables: {
        offer,
      },
    })

    console.log('answer', data.createVideoSDP)
    p.signal(data.createVideoSDP)

    p.on('connect', () => {
      console.log('CONNECT')
    })

    const stream = await new Promise(resolve => p.on('stream', resolve))
    console.log({ stream })

    // got remote video stream, now let's show it in a video tag
    if ('srcObject' in videoEl.current) {
      videoEl.current.srcObject = stream
    } else {
      videoEl.current.src = window.URL.createObjectURL(stream) // for older browsers
    }

    try {
      videoEl.current.play()
    } catch (e) {
      console.error(e)
    }
  }, [])

  const { isLoading, error } = useAsync({
    promiseFn: loadVideo,
  })

  if (error || peerError) {
    throw error || peerError
  }

  const nextProps = {
    ...props,
    videoEl,
    isLoading,
  }

  return (
    <Component {...nextProps} />
  )
}

const VideoStreamer = ({
  videoEl,
  isLoading,
}) => {
  const classes = useStyles()

  return (
    <LoadingOverlay loading={isLoading}>
      <div className={classes.container}>
        <video
          ref={videoEl}
          className={classes.video}
          controls="controls"
        >
        </video>
      </div>
    </LoadingOverlay>
  )
}

export const Component = VideoStreamer
export default enhance(VideoStreamer)
