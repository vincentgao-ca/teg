import { useState } from 'react'
import { useSubscription } from 'react-apollo-hooks'
import jsonpatch from 'json-patch'

const useLiveSubscription = (subscription, options = {}) => {
  const [counter, setRenderCounter] = useState(0)
  const [state, setState] = useState()

  const onSubscriptionData = (event) => {
    const { query, patch } = event.subscriptionData.data.live
    let nextState = state

    if (query != null) {
      nextState = query
    }

    if (patch != null) {
      patch.forEach((patchOp) => {
        nextState = jsonpatch.apply(nextState, patchOp)
      })
    }

    setState(nextState)
    // force a re-render for each response
    setRenderCounter(counter + 1)

    if (options.onSubscriptionData != null) {
      options.onSubscriptionData({
        ...options,
        subscriptionData: {
          data: nextState,
        },
      })
    }
  }

  const { error } = useSubscription(subscription, {
    ...options,
    onSubscriptionData,
  })

  return {
    error,
    loading: state == null && error == null,
    data: state,
  }
}

export default useLiveSubscription