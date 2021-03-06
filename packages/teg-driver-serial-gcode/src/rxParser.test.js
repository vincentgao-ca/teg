import rxParser from './rxParser'
import * as marlin from './marlinFixture'

test('parses greetings', () => {
  const result = rxParser('start')

  expect(result).toEqual({
    type: 'greeting',
    raw: 'start',
  })
})

test('parses resends', () => {
  const result = rxParser('rs N:95')

  expect(result).toEqual({
    type: 'resend',
    raw: 'rs N:95',
    lineNumber: 95,
  })
})

test('parses checksum errors as warnings', () => {
  const raw = marlin.errors.checksumMismatch[0]
  const result = rxParser(raw)

  expect(result).toEqual({
    type: 'warning',
    message: 'checksum mismatch, Last Line: 0',
    raw,
  })
})

test('parses echo lines', () => {
  const result = rxParser('echo:stuff')

  expect(result).toEqual({
    type: 'echo',
    raw: 'echo:stuff',
  })
})

test('parses debug lines', () => {
  const result = rxParser('debug_ stuff')

  expect(result).toEqual({
    type: 'debug',
    raw: 'debug_ stuff',
  })
})

test('parses errors', () => {
  const result = rxParser('error:stuff')

  expect(result).toEqual({
    type: 'error',
    message: 'stuff',
    raw: 'error:stuff',
  })
})

test('parses position feedback', () => {
  const raw = 'X:10 Y:20.2 Z:30.03 E:0.00 Count X: 0.00Y:0.00Z:0.00'
  const result = rxParser(raw)

  expect(result).toEqual({
    type: 'feedback',
    raw,
    position: {
      x: 10,
      y: 20.2,
      z: 30.03,
    },
    targetTemperaturesCountdown: null,
    temperatures: {},
  })
})

describe('parses oks', () => {
  test('without heater values', () => {
    const result = rxParser('ok')

    expect(result).toEqual({
      type: 'ok',
      raw: 'ok',
    })
  })

  const malformedOKs = ['okok', 'kok', 'ook']

  malformedOKs.forEach((raw) => {
    test(`from '${raw}'`, () => {
      const result = rxParser(raw)

      expect(result).toEqual({
        type: 'ok',
        raw,
      })
    })
  })

  test('with heater values', () => {
    const result = rxParser('ok t: 42 e3: 200')

    expect(result).toEqual({
      type: 'ok',
      raw: 'ok t: 42 e3: 200',
      position: null,
      temperatures: {
        e0: 42,
        e3: 200,
      },
      targetTemperaturesCountdown: null,
    })
  })

  test('with a w value', () => {
    const result = rxParser('ok t: 42 e3: 200 w: 29')

    expect(result).toEqual({
      type: 'ok',
      raw: 'ok t: 42 e3: 200 w: 29',
      position: null,
      temperatures: {
        e0: 42,
        e3: 200,
      },
      targetTemperaturesCountdown: 29000,
    })
  })
})

describe('parses temperature feedback', () => {
  test('with heater values but without a space', () => {
    const result = rxParser('t: 42 e3: 200')

    expect(result).toEqual({
      type: 'feedback',
      raw: 't: 42 e3: 200',
      position: null,
      temperatures: {
        e0: 42,
        e3: 200,
      },
      targetTemperaturesCountdown: null,
    })
  })

  test('with heater values', () => {
    const result = rxParser(' t: 42 e3: 200 w: 5')

    expect(result).toEqual({
      type: 'feedback',
      raw: ' t: 42 e3: 200 w: 5',
      position: null,
      temperatures: {
        e0: 42,
        e3: 200,
      },
      targetTemperaturesCountdown: 5000,
    })
  })
})
