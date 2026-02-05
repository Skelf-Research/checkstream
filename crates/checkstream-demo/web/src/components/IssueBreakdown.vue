<script setup lang="ts">
import { computed } from 'vue'
import { useEventsStore } from '../stores/events'

const eventsStore = useEventsStore()

const issues = computed(() => {
  const counts = eventsStore.issueCounts
  return [
    { type: 'pii', label: 'PII Detected', count: counts.pii || 0, color: 'text-orange-400', bg: 'bg-orange-400' },
    { type: 'toxicity', label: 'Toxicity', count: counts.toxicity || 0, color: 'text-red-400', bg: 'bg-red-400' },
    { type: 'prompt_injection', label: 'Prompt Injection', count: counts.prompt_injection || 0, color: 'text-purple-400', bg: 'bg-purple-400' },
    { type: 'financial_advice', label: 'Financial Advice', count: counts.financial_advice || 0, color: 'text-green-400', bg: 'bg-green-400' },
  ]
})

const total = computed(() => issues.value.reduce((sum, i) => sum + i.count, 0))
</script>

<template>
  <div class="bg-gray-800 rounded-lg p-6">
    <h2 class="text-lg font-semibold mb-4 text-gray-200">Issue Breakdown</h2>

    <div class="space-y-4">
      <div
        v-for="issue in issues"
        :key="issue.type"
        class="flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <div :class="['w-3 h-3 rounded-full', issue.bg]"></div>
          <span class="text-sm text-gray-300">{{ issue.label }}</span>
        </div>
        <div class="flex items-center gap-2">
          <span :class="['font-mono text-lg font-semibold', issue.color]">
            {{ issue.count }}
          </span>
          <span class="text-xs text-gray-500">
            {{ total > 0 ? ((issue.count / total) * 100).toFixed(0) : 0 }}%
          </span>
        </div>
      </div>
    </div>

    <!-- Visual bar representation -->
    <div class="mt-4">
      <div class="h-3 bg-gray-700 rounded-full overflow-hidden flex" v-if="total > 0">
        <div
          v-for="issue in issues"
          :key="issue.type"
          :class="[issue.bg, 'h-full transition-all duration-300']"
          :style="{ width: `${(issue.count / total) * 100}%` }"
        ></div>
      </div>
      <div v-else class="h-3 bg-gray-700 rounded-full"></div>
    </div>

    <div class="mt-3 text-center text-sm text-gray-400">
      {{ total }} total issues detected
    </div>
  </div>
</template>
