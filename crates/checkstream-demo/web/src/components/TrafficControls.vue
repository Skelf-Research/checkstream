<script setup lang="ts">
import { useTrafficStore } from '../stores/traffic'
import { useEventsStore } from '../stores/events'

const trafficStore = useTrafficStore()
const eventsStore = useEventsStore()
</script>

<template>
  <div class="bg-gray-800 rounded-lg p-6">
    <h2 class="text-lg font-semibold mb-4 text-gray-200">Traffic Generator</h2>

    <!-- Rate slider -->
    <div class="mb-4">
      <div class="flex justify-between text-sm text-gray-400 mb-2">
        <span>Rate</span>
        <span class="font-mono">{{ trafficStore.rate }} req/s</span>
      </div>
      <input
        type="range"
        v-model.number="trafficStore.rate"
        min="1"
        max="50"
        class="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-500"
        :disabled="trafficStore.running"
      />
    </div>

    <!-- Issue toggles -->
    <div class="space-y-3 mb-4">
      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          v-model="trafficStore.issueConfig.pii_enabled"
          class="w-4 h-4 rounded bg-gray-700 border-gray-600 text-orange-500 focus:ring-orange-500"
        />
        <span class="text-sm text-gray-300">Inject PII</span>
        <span class="text-xs text-orange-400 bg-orange-400/10 px-2 py-0.5 rounded">SSN, Cards, Email</span>
      </label>

      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          v-model="trafficStore.issueConfig.toxicity_enabled"
          class="w-4 h-4 rounded bg-gray-700 border-gray-600 text-red-500 focus:ring-red-500"
        />
        <span class="text-sm text-gray-300">Inject Toxicity</span>
        <span class="text-xs text-red-400 bg-red-400/10 px-2 py-0.5 rounded">Offensive content</span>
      </label>

      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          v-model="trafficStore.issueConfig.injection_enabled"
          class="w-4 h-4 rounded bg-gray-700 border-gray-600 text-purple-500 focus:ring-purple-500"
        />
        <span class="text-sm text-gray-300">Inject Prompt Injection</span>
        <span class="text-xs text-purple-400 bg-purple-400/10 px-2 py-0.5 rounded">Jailbreak attempts</span>
      </label>

      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          v-model="trafficStore.issueConfig.financial_advice_enabled"
          class="w-4 h-4 rounded bg-gray-700 border-gray-600 text-green-500 focus:ring-green-500"
        />
        <span class="text-sm text-gray-300">Inject Financial Advice</span>
        <span class="text-xs text-green-400 bg-green-400/10 px-2 py-0.5 rounded">Investment tips</span>
      </label>
    </div>

    <!-- Buttons -->
    <div class="flex gap-2">
      <button
        @click="trafficStore.toggle()"
        :class="[
          'flex-1 py-2.5 px-4 rounded-lg font-medium text-sm transition-all',
          trafficStore.running
            ? 'bg-red-600 hover:bg-red-700 text-white'
            : 'bg-green-600 hover:bg-green-700 text-white'
        ]"
      >
        {{ trafficStore.running ? 'Stop Traffic' : 'Start Traffic' }}
      </button>

      <button
        @click="eventsStore.clearEvents(); trafficStore.resetMetrics()"
        class="py-2.5 px-4 rounded-lg font-medium text-sm bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
      >
        Reset
      </button>
    </div>
  </div>
</template>
