<script setup lang="ts">
import { computed, ref } from 'vue'
import { useEventsStore, type RequestResult } from '../stores/events'

const emit = defineEmits<{
  select: [id: string]
}>()

const eventsStore = useEventsStore()

const filterAction = ref<'all' | 'pass' | 'block' | 'redact'>('all')
const filterIssue = ref<string>('all')

const filteredEvents = computed(() => {
  return eventsStore.recentEvents.filter((event) => {
    if (filterAction.value !== 'all' && event.action !== filterAction.value) {
      return false
    }
    if (filterIssue.value !== 'all') {
      const hasIssue = event.issues_detected.some((i) => {
        const normalized = ['ssn', 'credit_card', 'email', 'phone'].includes(i.issue_type)
          ? 'pii'
          : i.issue_type
        return normalized === filterIssue.value
      })
      if (!hasIssue) return false
    }
    return true
  })
})

const actionColor = (action: string) => {
  switch (action) {
    case 'pass':
      return 'text-green-400 bg-green-400/10'
    case 'block':
      return 'text-red-400 bg-red-400/10'
    case 'redact':
      return 'text-yellow-400 bg-yellow-400/10'
    default:
      return 'text-gray-400 bg-gray-400/10'
  }
}

const formatTime = (timestamp: string) => {
  return new Date(timestamp).toLocaleTimeString()
}

const truncate = (text: string, maxLength: number) => {
  if (text.length <= maxLength) return text
  return text.slice(0, maxLength) + '...'
}
</script>

<template>
  <div>
    <!-- Filters -->
    <div class="flex items-center gap-4 mb-4">
      <div class="flex items-center gap-2">
        <span class="text-sm text-gray-400">Action:</span>
        <select
          v-model="filterAction"
          class="bg-gray-700 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="all">All</option>
          <option value="pass">Pass</option>
          <option value="block">Block</option>
          <option value="redact">Redact</option>
        </select>
      </div>

      <div class="flex items-center gap-2">
        <span class="text-sm text-gray-400">Issue:</span>
        <select
          v-model="filterIssue"
          class="bg-gray-700 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="all">All</option>
          <option value="pii">PII</option>
          <option value="toxicity">Toxicity</option>
          <option value="prompt_injection">Prompt Injection</option>
          <option value="financial_advice">Financial Advice</option>
        </select>
      </div>

      <div class="ml-auto text-sm text-gray-400">
        {{ filteredEvents.length }} events
      </div>
    </div>

    <!-- Event list -->
    <div class="h-96 overflow-y-auto space-y-2">
      <div
        v-for="event in filteredEvents"
        :key="event.id"
        @click="emit('select', event.id)"
        class="bg-gray-700/50 rounded-lg p-3 cursor-pointer hover:bg-gray-700 transition-colors fade-in"
      >
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-3">
            <span
              :class="[
                'px-2 py-0.5 rounded text-xs font-medium uppercase',
                actionColor(event.action)
              ]"
            >
              {{ event.action }}
            </span>
            <span class="text-xs text-gray-400 font-mono">
              {{ formatTime(event.timestamp) }}
            </span>
            <span class="text-xs text-gray-500">
              {{ event.latency_ms.toFixed(1) }}ms
            </span>
          </div>
          <div class="flex items-center gap-1">
            <span
              v-for="issue in event.issues_detected"
              :key="issue.issue_type"
              class="text-xs px-1.5 py-0.5 rounded bg-gray-600 text-gray-300"
            >
              {{ issue.issue_type }}
            </span>
          </div>
        </div>
        <div class="text-sm text-gray-300 font-mono truncate">
          {{ truncate(event.request_preview, 80) }}
        </div>
        <div
          v-if="event.response_preview"
          class="text-xs text-gray-400 mt-1 truncate"
        >
          Response: {{ truncate(event.response_preview, 60) }}
        </div>
      </div>

      <div
        v-if="filteredEvents.length === 0"
        class="text-center text-gray-500 py-8"
      >
        <p>No events yet.</p>
        <p class="text-sm mt-2">Start traffic generation to see events appear here.</p>
      </div>
    </div>
  </div>
</template>
