import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface DetectedIssue {
  issue_type: string
  classifier: string
  score: number
  matched_text?: string
  span?: [number, number]
}

export interface RequestResult {
  id: string
  timestamp: string
  action: 'pass' | 'block' | 'redact'
  latency_ms: number
  phase: string
  issues_detected: DetectedIssue[]
  triggered_rules: string[]
  request_preview: string
  response_preview?: string
}

export interface MetricsSnapshot {
  timestamp: string
  requests_total: number
  requests_per_minute: number
  block_rate: number
  redact_rate: number
  avg_latency_ms: number
  p99_latency_ms: number
  active_connections: number
}

type DemoEvent =
  | { type: 'RequestCompleted'; data: RequestResult }
  | { type: 'MetricsUpdate'; data: MetricsSnapshot }
  | { type: 'TrafficStateChanged'; data: 'running' | 'stopped' | 'paused' }
  | { type: 'ConfigChanged'; data: { field: string; old_value: unknown; new_value: unknown } }
  | { type: 'Error'; data: { message: string; code?: string } }

export const useEventsStore = defineStore('events', () => {
  const events = ref<RequestResult[]>([])
  const metrics = ref<MetricsSnapshot | null>(null)
  const connected = ref(false)
  const trafficState = ref<'running' | 'stopped' | 'paused'>('stopped')

  let ws: WebSocket | null = null
  let reconnectTimeout: number | null = null

  const recentEvents = computed(() => events.value.slice(0, 100))

  const issueCounts = computed(() => {
    const counts: Record<string, number> = {}
    for (const event of events.value) {
      for (const issue of event.issues_detected) {
        const type = normalizeIssueType(issue.issue_type)
        counts[type] = (counts[type] || 0) + 1
      }
    }
    return counts
  })

  const actionCounts = computed(() => {
    const counts = { pass: 0, block: 0, redact: 0 }
    for (const event of events.value) {
      counts[event.action]++
    }
    return counts
  })

  function normalizeIssueType(type: string): string {
    if (['ssn', 'credit_card', 'email', 'phone'].includes(type)) {
      return 'pii'
    }
    return type
  }

  function connect() {
    if (ws?.readyState === WebSocket.OPEN) return

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    ws = new WebSocket(`${protocol}//${window.location.host}/ws`)

    ws.onopen = () => {
      connected.value = true
      console.log('WebSocket connected')
    }

    ws.onclose = () => {
      connected.value = false
      console.log('WebSocket disconnected')

      // Attempt to reconnect after 3 seconds
      if (reconnectTimeout) clearTimeout(reconnectTimeout)
      reconnectTimeout = window.setTimeout(() => connect(), 3000)
    }

    ws.onerror = (error) => {
      console.error('WebSocket error:', error)
    }

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as DemoEvent
        handleEvent(data)
      } catch (e) {
        console.error('Failed to parse event:', e)
      }
    }
  }

  function disconnect() {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
      reconnectTimeout = null
    }
    if (ws) {
      ws.close()
      ws = null
    }
  }

  function handleEvent(event: DemoEvent) {
    switch (event.type) {
      case 'RequestCompleted':
        events.value.unshift(event.data)
        if (events.value.length > 1000) {
          events.value.pop()
        }
        break

      case 'MetricsUpdate':
        metrics.value = event.data
        break

      case 'TrafficStateChanged':
        trafficState.value = event.data
        break

      case 'Error':
        console.error('Server error:', event.data.message)
        break
    }
  }

  function clearEvents() {
    events.value = []
  }

  return {
    events,
    metrics,
    connected,
    trafficState,
    recentEvents,
    issueCounts,
    actionCounts,
    connect,
    disconnect,
    clearEvents,
  }
})
