import { Record } from 'immutable'
/* eslint-disable no-bitwise */

export const PT_UINT32 = Record({
  isInt: true,
  maxLength: 5,
  signed: 0,
})()

export const PT_INT32 = PT_UINT32.merge({
  signed: 1,
})
export const PT_UINT16 = PT_UINT32.merge({
  maxLength: 3,
})
export const PT_INT16 = PT_UINT32.merge({
  signed: 1,
  maxLength: 3,
})
export const PT_BYTE = PT_UINT32.merge({
  maxLength: 2,
})

export const encodePTInt = (self, out, v) => {
  let nextOut = out
  if (v >= 0xc000000 || v < -0x4000000) {
    nextOut = nextOut.concat(((v >> 28) & 0x7f) | 0x80)
  }
  if (v >= 0x180000 || v < -0x80000) {
    nextOut = nextOut.concat(((v >> 21) & 0x7f) | 0x80)
  }
  if (v >= 0x3000 || v < -0x1000) {
    nextOut = nextOut.concat(((v >> 14) & 0x7f) | 0x80)
  }
  if (v >= 0x60 || v < -0x20) {
    nextOut = nextOut.concat(((v >> 7) & 0x7f) | 0x80)
  }
  nextOut = nextOut.concat(v & 0x7f)
  return nextOut
}

export const parsePTInt = (self, s, pos) => {
  let c = s[pos]
  let nextPos = pos
  nextPos += 1
  let v = c & 0x7f
  if ((c & 0x60) === 0x60) {
    v |= -0x20
  }
  while (c & 0x80) {
    c = s[nextPos]
    nextPos += 1
    v = (v << 7) | (c & 0x7f)
  }
  if (!self.signed) {
    v = Math.round(v & 0xffffffff)
  }
  return [v, nextPos]
}
