import { Record } from 'immutable'

export const PT_STRING = Record({
  isInt: false,
  maxLength: 64,
})

export const PR_PROGMEM_BUFFER = PT_STRING
export const PT_BUFFER = PT_STRING


export const encodePTString = (self, out, v) => {
  const lengthChar = String.fromCharCode(v.length)
  return out.concat(lengthChar, v)
}

export const parsePTString = (self, message, pos) => {
  const length = message[pos].charCodeAt(0)
  const nextPos = pos + length + 1
  const value = message.slice(pos + 1, nextPos)
  return [value, nextPos]
}
