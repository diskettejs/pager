import { describe, expect, test } from 'vitest'
import { Encoding } from '../index.js'

describe('Encoding', () => {
  test('default enconding', () => {
    expect(Encoding.default().toString()).equals(Encoding.zenohBytes().toString())
  })

  test('with schema', () => {
    const enconding1 = Encoding.from('text/plain;utf-8')
    const enconding2 = Encoding.textPlain().withSchema('utf-8')

    expect(enconding1.toString()).equals(enconding2.toString())
  })
})
