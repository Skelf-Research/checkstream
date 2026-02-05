import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface IssueConfig {
  pii_enabled: boolean
  pii_probability: number
  toxicity_enabled: boolean
  toxicity_probability: number
  injection_enabled: boolean
  injection_probability: number
  financial_advice_enabled: boolean
  financial_probability: number
}

export const useTrafficStore = defineStore('traffic', () => {
  const running = ref(false)
  const rate = ref(10)
  const issueConfig = ref<IssueConfig>({
    pii_enabled: true,
    pii_probability: 0.3,
    toxicity_enabled: true,
    toxicity_probability: 0.2,
    injection_enabled: false,
    injection_probability: 0.1,
    financial_advice_enabled: false,
    financial_probability: 0.1,
  })

  async function start() {
    try {
      const response = await fetch('/api/traffic/start', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          rate: rate.value,
          issue_config: issueConfig.value,
        }),
      })

      if (response.ok) {
        running.value = true
      } else {
        const error = await response.json()
        console.error('Failed to start traffic:', error)
      }
    } catch (e) {
      console.error('Failed to start traffic:', e)
    }
  }

  async function stop() {
    try {
      const response = await fetch('/api/traffic/stop', {
        method: 'POST',
      })

      if (response.ok) {
        running.value = false
      }
    } catch (e) {
      console.error('Failed to stop traffic:', e)
    }
  }

  async function toggle() {
    if (running.value) {
      await stop()
    } else {
      await start()
    }
  }

  async function updateIssueConfig() {
    try {
      await fetch('/api/config/issues', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(issueConfig.value),
      })
    } catch (e) {
      console.error('Failed to update issue config:', e)
    }
  }

  async function resetMetrics() {
    try {
      await fetch('/api/stats/reset', { method: 'POST' })
    } catch (e) {
      console.error('Failed to reset metrics:', e)
    }
  }

  return {
    running,
    rate,
    issueConfig,
    start,
    stop,
    toggle,
    updateIssueConfig,
    resetMetrics,
  }
})
