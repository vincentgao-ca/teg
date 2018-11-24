import {
  PT_STRING,
  PR_PROGMEM_BUFFER,
  PT_BUFFER,
  encodePTString,
  parsePTString,
} from './stringAndBuffers'

import {
  PT_UINT32,
  PT_INT32,
  PT_UINT16,
  PT_INT16,
  PT_BYTE,
  encodePTInt,
  parsePTInt,
} from './variableLengthQuantity'

const MESSAGE_TYPES = {
  '%u': PT_UINT32,
  '%i': PT_INT32,
  '%hu': PT_UINT16,
  '%hi': PT_INT16,
  '%c': PT_BYTE,
  '%s': PT_STRING,
  '%.*s': PR_PROGMEM_BUFFER,
  '%*s': PT_BUFFER,
}

const defaultMessages = {
  0: 'identify_response offset=%u data=%.*s',
  1: 'identify offset=%u count=%c',
}

const MESSAGE_MIN = 5
const MESSAGE_MAX = 64
const MESSAGE_HEADER_SIZE  = 2
const MESSAGE_TRAILER_SIZE = 3
const MESSAGE_POS_LEN = 0
const MESSAGE_POS_SEQ = 1
const MESSAGE_TRAILER_CRC  = 3
const MESSAGE_TRAILER_SYNC = 1
const MESSAGE_PAYLOAD_MAX = MESSAGE_MAX - MESSAGE_MIN
const MESSAGE_SEQ_MASK = 0x0f
const MESSAGE_DEST = 0x10
const MESSAGE_SYNC = '\x7E'

const crc16CCITT = (buf) => {
  /* eslint-disable no-bitwise, operator-assignment */
  let crc = 0xffff
  for (const char of buf) {
    let data = char.charCodeAt(0)
    data = data ^ (crc & 0xff)
    data ^= (data & 0x0f) << 4
    crc = ((data << 8) | (crc >> 8)) ^ (data >> 4) ^ (data << 3)
  }
  crc = String.fromCharCode(crc >> 8, crc & 0xff)
  return crc
}

const MessageFormatDef = Record({
  id: null,
  msgFormat: null,
  name: null,
  params: List(),
})

const ParamDef = Record({
  key: null,
  type: null,
})

const parseMessageFormatDef = (id, msgFormat) => {
  const parts = msgFormat.split()

  const params = parts.slice(1)
    .map(argFormat => argFormat.split('='))
    .map(([key, typeString]) => ParamDef({
      key,
      type: MESSAGE_TYPES[typeString]
    }))
  const entry = MessageFormatDef({
    id,
    msgFormat,
    name: parts[0],
    params,
  })
}

/*
 * takes a messageFormatDef and an object of parameters and returns a binary
 * message
 */
const encodeMessage = (messageFormatDef, params) => {
  let out = []
  out.push(messageFormatDef.id)
  messageFormatDef.params.forEach(({ key, type }) => {
    const value = params[key]
    if (type.isInt) {
      out = encodePTInt(out, value)
    }
    else {
      out = encodePTString(out, value)
    }
  })
  return out
}

const parseMessageParams = (messageFormatDef, message, pos) => {
  let nextPos = pos + 1
  const out = {}
  messageFormatDef.params.forEach(({ key, type }) => {
    if (type.isInt) {
      [out[key], nextPos] = parsePTInt(message, nextPos)
    }
    else {
      [out[key], nextPos] = parsePTString(message, nextPos)
    }
  })
  return [out, nextPos]
}
