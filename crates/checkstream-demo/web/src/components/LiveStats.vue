<script setup lang="ts">
import { computed } from 'vue'
import { useEventsStore } from '../stores/events'

const eventsStore = useEventsStore()

const stats = computed(() => {
  const m = eventsStore.metrics
  return {
    rpm: m?.requests_per_minute?.toFixed(1) ?? '0',
    blockRate: m ? (m.block_rate * 100).toFixed(1) : '0',
    redactRate: m ? (m.redact_rate * 100).toFixed(1) : '0',
    avgLatency: m?.avg_latency_ms?.toFixed(1) ?? '0',
    p99Latency: m?.p99_latency_ms?.toFixed(1) ?? '0',
    total: m?.requests_total ?? 0,
  }
})

const counts = computed(() => eventsStore.actionCounts)
</script>

<template>
  <div class="bg-gray-800 rounded-lg p-6">
    <h2 class="text-lg font-semibold mb-4 text-gray-200">Live Statistics</h2>

    <div class="grid grid-cols-2 gap-4">
      <!-- Requests per minute -->
      <div class="bg-gray-700/50 rounded-lg p-4">
        <div class="text-3xl font-bold text-blue-400">{{ stats.rpm }}</div>
        <div class="text-sm text-gray-400">req/min</div>
      </div>

      <!-- Block rate -->
      <div class="bg-gray-700/50 rounded-lg p-4">
        <div class="text-3xl font-bold text-red-400">{{ stats.blockRate }}%</div>
        <div class="text-sm text-gray-400">blocked</div>
      </div>

      <!-- Average latency -->
      <div class="bg-gray-700/50 rounded-lg p-4">
        <div class="text-3xl font-bold text-yellow-400">{{ stats.avgLatency }}ms</div>
        <div class="text-sm text-gray-400">avg latency</div>
      </div>

      <!-- Total requests -->
      <div class="bg-gray-700/50 rounded-lg p-4">
        <div class="text-3xl font-bold text-gray-200">{{ stats.total }}</div>
        <div class="text-sm text-gray-400">total</div>
      </div>
    </div>

    <!-- Action breakdown bar -->
    <div class="mt-4">
      <div class="flex justify-between text-xs text-gray-400 mb-1">
        <span>Action breakdown</span>
        <span>{{ counts.pass + counts.block + counts.redact }} total</span>
      </div>
      <div class="h-2 bg-gray-700 rounded-full overflow-hidden flex">
        <div
          class="bg-green-500 h-full transition-all duration-300"
          :style="{ width: `${(counts.pass / (counts.pass + counts.block + counts.redact || 1)) * 100}%` }"
        ></div>
        <div
          class="bg-red-500 h-full transition-all duration-300"
          :style="{ width: `${(counts.block / (counts.pass + counts.block + counts.redact || 1)) * 100}%` }"
        ></div>
        <div
          class="bg-yellow-500 h-full transition-all duration-300"
          :style="{ width: `${(counts.redact / (counts.pass + counts.block + counts.redact || 1)) * 100}%` }"
        ></div>
      </div>
      <div class="flex justify-between text-xs mt-1">
        <span class="text-green-400">Pass: {{ counts.pass }}</span>
        <span class="text-red-400">Block: {{ counts.block }}</span>
        <span class="text-yellow-400">Redact: {{ counts.redact }}</span>
      </div>
    </div>
  </div>
</template>
