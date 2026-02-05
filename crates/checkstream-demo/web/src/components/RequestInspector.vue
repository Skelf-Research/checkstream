<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useEventsStore, type RequestResult } from '../stores/events'

const props = defineProps<{
  eventId: string | null
}>()

const eventsStore = useEventsStore()

const event = computed<RequestResult | null>(() => {
  if (!props.eventId) return null
  return eventsStore.events.find((e) => e.id === props.eventId) || null
})

const activeSection = ref<'request' | 'response' | 'issues'>('request')

const actionColor = (action: string) => {
  switch (action) {
    case 'pass':
      return 'text-green-400'
    case 'block':
      return 'text-red-400'
    case 'redact':
      return 'text-yellow-400'
    default:
      return 'text-gray-400'
  }
}

const formatTime = (timestamp: string) => {
  return new Date(timestamp).toLocaleString()
}

const issueTypeColor = (type: string) => {
  if (['ssn', 'credit_card', 'email', 'phone'].includes(type) || type === 'pii') {
    return 'text-orange-400 bg-orange-400/10'
  }
  switch (type) {
    case 'toxicity':
      return 'text-red-400 bg-red-400/10'
    case 'prompt_injection':
      return 'text-purple-400 bg-purple-400/10'
    case 'financial_advice':
      return 'text-green-400 bg-green-400/10'
    default:
      return 'text-gray-400 bg-gray-400/10'
  }
}
</script>

<template>
  <div>
    <div v-if="event" class="space-y-4">
      <!-- Header -->
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <span
            :class="['text-2xl font-bold uppercase', actionColor(event.action)]"
          >
            {{ event.action }}
          </span>
          <span class="text-sm text-gray-400">
            {{ formatTime(event.timestamp) }}
          </span>
          <span class="text-sm text-gray-500 font-mono">
            ID: {{ event.id.slice(0, 8) }}
          </span>
        </div>
        <div class="flex items-center gap-2 text-sm">
          <span class="text-gray-400">Latency:</span>
          <span class="font-mono text-yellow-400">{{ event.latency_ms.toFixed(2) }}ms</span>
        </div>
      </div>

      <!-- Tab navigation -->
      <div class="flex border-b border-gray-700">
        <button
          @click="activeSection = 'request'"
          :class="[
            'px-4 py-2 text-sm font-medium transition-colors',
            activeSection === 'request'
              ? 'text-blue-400 border-b-2 border-blue-400'
              : 'text-gray-400 hover:text-white'
          ]"
        >
          Request
        </button>
        <button
          @click="activeSection = 'response'"
          :class="[
            'px-4 py-2 text-sm font-medium transition-colors',
            activeSection === 'response'
              ? 'text-blue-400 border-b-2 border-blue-400'
              : 'text-gray-400 hover:text-white'
          ]"
        >
          Response
        </button>
        <button
          @click="activeSection = 'issues'"
          :class="[
            'px-4 py-2 text-sm font-medium transition-colors',
            activeSection === 'issues'
              ? 'text-blue-400 border-b-2 border-blue-400'
              : 'text-gray-400 hover:text-white'
          ]"
        >
          Issues ({{ event.issues_detected.length }})
        </button>
      </div>

      <!-- Content sections -->
      <div class="bg-gray-700/30 rounded-lg p-4">
        <!-- Request section -->
        <div v-if="activeSection === 'request'">
          <h3 class="text-sm font-medium text-gray-400 mb-2">User Message</h3>
          <pre class="text-sm text-gray-200 whitespace-pre-wrap font-mono bg-gray-800 rounded p-4">{{ event.request_preview }}</pre>
        </div>

        <!-- Response section -->
        <div v-else-if="activeSection === 'response'">
          <h3 class="text-sm font-medium text-gray-400 mb-2">Assistant Response</h3>
          <pre
            v-if="event.response_preview"
            class="text-sm text-gray-200 whitespace-pre-wrap font-mono bg-gray-800 rounded p-4"
          >{{ event.response_preview }}</pre>
          <div v-else class="text-gray-500 text-center py-4">
            No response (request was blocked)
          </div>
        </div>

        <!-- Issues section -->
        <div v-else-if="activeSection === 'issues'">
          <div v-if="event.issues_detected.length > 0" class="space-y-3">
            <div
              v-for="(issue, idx) in event.issues_detected"
              :key="idx"
              class="bg-gray-800 rounded-lg p-4"
            >
              <div class="flex items-center justify-between mb-2">
                <span
                  :class="['px-2 py-1 rounded text-sm font-medium', issueTypeColor(issue.issue_type)]"
                >
                  {{ issue.issue_type }}
                </span>
                <span class="text-sm text-gray-400">
                  Score: <span class="font-mono text-white">{{ (issue.score * 100).toFixed(0) }}%</span>
                </span>
              </div>
              <div class="text-sm text-gray-400">
                Classifier: <span class="font-mono text-gray-300">{{ issue.classifier }}</span>
              </div>
              <div v-if="issue.matched_text" class="mt-2">
                <div class="text-xs text-gray-500 mb-1">Matched text:</div>
                <code class="text-sm text-red-300 bg-red-900/30 px-2 py-1 rounded">
                  {{ issue.matched_text }}
                </code>
              </div>
            </div>
          </div>
          <div v-else class="text-gray-500 text-center py-4">
            No issues detected
          </div>

          <!-- Triggered rules -->
          <div v-if="event.triggered_rules.length > 0" class="mt-4">
            <h3 class="text-sm font-medium text-gray-400 mb-2">Triggered Rules</h3>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="rule in event.triggered_rules"
                :key="rule"
                class="px-2 py-1 bg-blue-500/20 text-blue-300 rounded text-sm"
              >
                {{ rule }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- No selection state -->
    <div v-else class="text-center text-gray-500 py-12">
      <p class="text-lg mb-2">No request selected</p>
      <p class="text-sm">Click on an event in the Event Log to inspect it here.</p>
    </div>
  </div>
</template>
