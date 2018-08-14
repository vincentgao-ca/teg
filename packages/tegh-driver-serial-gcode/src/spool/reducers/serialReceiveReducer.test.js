// @flow
import { driverError } from 'tegh-server'
import { utils as sagaUtils } from 'redux-saga'

import { despoolTask } from 'tegh-server'

import delayMockedSagaTester from '../../test_helpers/delayMockedSagaTester'
import serialSend from '../../serial/actions/serialSend'
import serialReceiveSaga from './serialReceiveSaga'

const { SAGA_ACTION } = sagaUtils

const serialReceive = (type, data) => ({
  type: SERIAL_RECEIVE,
  data: {
    type,
    ...data,
  },
})

const despoolAction = {
  ...despoolTask(),
  [SAGA_ACTION]: true,
}

const itDoesNothingWhen = ({ ready, type, ignoreOK = false }) => {
  it(
    `when ready = ${ready} and shouldIgnoreOK = ${ignoreOK}`
    + ' it does nothing',
    () => {
      const selectors = {
        isReady: () => ready,
        shouldIgnoreOK: () => ignoreOK,
        getCurrentFileName: () => 'test.gcode',
        getCurrentFileLineNumber: () => 1337,
      }
      const { sagaTester, delayMock } = delayMockedSagaTester({
        initialState: {},
        saga: serialReceiveSaga(selectors),
      })
      sagaTester.dispatch(serialReceive(type))

      const pause = delayMock.unacknowledgedDelay
      expect(pause).toBe(null)
      const result = sagaTester.getCalledActions().slice(1)
      expect(result).toEqual([])
    },
  )
}

describe('SERIAL_RECEIVE ok', () => {
  itDoesNothingWhen({ ready: false, type: 'ok' })
  itDoesNothingWhen({ ready: true, ignoreOK: true, type: 'ok' })
  it(
    'when driver is ready it dispatching DESPOOL',
    () => {
      const selectors = {
        isReady: () => true,
        shouldIgnoreOK: () => false,
        getCurrentFileName: () => 'test.gcode',
        getCurrentFileLineNumber: () => 1337,
      }

      const { sagaTester, delayMock } = delayMockedSagaTester({
        initialState: {},
        saga: serialReceiveSaga(selectors),
      })
      sagaTester.dispatch(serialReceive('ok'))

      const result = sagaTester.getCalledActions()

      expect(result).toMatchObject([
        serialReceive('ok'),
        despoolAction,
      ])
    },
  )
})

describe('SERIAL_RECEIVE error', () => {
  itDoesNothingWhen({ ready: false, type: 'error' })
  it(
    'when driver is ready it dispatching DRIVER_ERROR',
    () => {
      const selectors = {
        isReady: () => true,
        shouldIgnoreOK: () => false,
        getCurrentFileName: () => 'test.gcode',
        getCurrentFileLineNumber: () => 1337,
      }
      const raw = 'Error:PROBE FAIL CLEAN NOZZLE'
      const serialReceiveError = serialReceive('error', { raw })
      const firmwareError = {
        ...driverError({
          code: 'FIRMWARE_ERROR',
          message: `test.gcode:1337: ${raw}`,
        }),
        [SAGA_ACTION]: true,
      }

      const { sagaTester, delayMock } = delayMockedSagaTester({
        initialState: {},
        saga: serialReceiveSaga(selectors),
      })
      sagaTester.dispatch(serialReceiveError)

      const result = sagaTester.getCalledActions()

      expect(result).toMatchObject([
        serialReceiveError,
        firmwareError,
      ])
    },
  )
})

describe('SERIAL_RECEIVE resend', () => {
  const requestResend = ({ requestLineNumber, currentLineNumber, onError }) => {
    let ignoreOK = false
    const selectors = {
      isReady: () => true,
      shouldIgnoreOK: () => ignoreOK,
      getCurrentLine: () => '(╯°□°）╯︵ ┻━┻',
      getCurrentSerialLineNumber: () => currentLineNumber,
      getCurrentFileName: () => 'test.gcode',
      getCurrentFileLineNumber: () => 1337,
    }
    const { sagaTester, delayMock } = delayMockedSagaTester({
      initialState: {},
      saga: serialReceiveSaga(selectors),
      options: {
        onError,
      },
    })
    const receiveResend = serialReceive('resend', {
      lineNumber: requestLineNumber,
    })
    sagaTester.dispatch(receiveResend)
    ignoreOK = true

    return { sagaTester, receiveResend }
  }

  itDoesNothingWhen({ ready: false, type: 'resend' })

  it(
    'when the resend is not for the previous line number it errors',
    () => {
      let errored = false
      const { sagaTester } = requestResend({
        requestLineNumber: 32,
        currentLineNumber: 41,
        onError: () => { errored = true },
      })

      expect(errored).toBe(false)
      sagaTester.dispatch(serialReceive('ok'))
      expect(errored).toBe(true)
    },
  )
  it(
    'when the resend is for the previous line number it resends it',
    () => {
      const { sagaTester, receiveResend } = requestResend({
        requestLineNumber: 41,
        currentLineNumber: 42,
      })

      sagaTester.dispatch(serialReceive('ok'))

      const result = sagaTester.getCalledActions()

      expect(result).toMatchObject([
        receiveResend,
        serialReceive('ok'),
        serialSend('(╯°□°）╯︵ ┻━┻', { lineNumber: 41 }),
      ])
    },
  )
})
