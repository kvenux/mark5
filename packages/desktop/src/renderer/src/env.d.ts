import type { Mark5API } from '@shared/types'

declare global {
  interface Window {
    mark5: Mark5API
  }
}

export {}
