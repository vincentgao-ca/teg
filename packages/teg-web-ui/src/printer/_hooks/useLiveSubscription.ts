import { useState, useEffect } from 'react'

import { useSubscription } from 'react-apollo-hooks'
import jsonpatch from 'json-patch'
import { useAsync } from 'react-async'

const useLiveSubscription = (
  subscription: any,
  options: any = {},
): {
  data: any,
  loading: boolean,
  error: any,
} => {
  const [counter, setRenderCounter] = useState(0)
  const [state, setState] = useState()

  // /*
  //  * A promise to emulate React.lazy and trigger React.suspension while
  //  * the initial query response loads.
  //  */
  // const [suspension] = useState(() => {
  //   const initialSuspension = {}
  //   initialSuspension.promise = new Promise((resolve, reject) => {
  //     initialSuspension.resolve = resolve
  //     initialSuspension.reject = reject
  //   })
  //
  //   return initialSuspension
  // })

  const onSubscriptionData = useAsync({
    deferFn: async (args: [{ subscriptionData: any }]) => {
      const { query, patch } = args[0].subscriptionData.data.live
      let nextState = state

      if (query != null) {
        nextState = query
      }

      if (patch != null) {
        if (patch.length === 0) return
        patch.forEach((patchOp) => {
          nextState = jsonpatch.apply(nextState, patchOp)
        })
      }

      setState(nextState)
      // force a re-render for each response
      setRenderCounter(val => val + 1)

      if (options.onSubscriptionData != null) {
        options.onSubscriptionData({
          ...options,
          subscriptionData: {
            data: nextState,
          },
        })
      }

      // if (query != null) {
      //   suspension.resolve()
      // }
    },
  })

  const { error } = useSubscription(subscription, {
    ...options,
    onSubscriptionData: onSubscriptionData.run,
  })

  const anyError = onSubscriptionData.error || error

  useEffect(() => {
    if (anyError != null) {
      throw anyError
      // suspension.resolve()
    }
  }, [anyError])

  // useEffect(() => { throw new Error('wat') }, [])

  const loading = state == null && error == null

  // if (loading) {
  //   throw suspension.promise
  // }

  return {
    error,
    loading,
    data: state,
    // suspensionPromise: suspension.promise,
  }
}

export default useLiveSubscription
