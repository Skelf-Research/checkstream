<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue'
import { useEventsStore } from './stores/events'
import { useTrafficStore } from './stores/traffic'
import LiveStats from './components/LiveStats.vue'
import TrafficControls from './components/TrafficControls.vue'
import IssueBreakdown from './components/IssueBreakdown.vue'
import EventLog from './components/EventLog.vue'
import RequestInspector from './components/RequestInspector.vue'

const eventsStore = useEventsStore()
const trafficStore = useTrafficStore()

const activeTab = ref<'log' | 'inspector'>('log')
const selectedEventId = ref<string | null>(null)

onMounted(() => {
  eventsStore.connect()
})

onUnmounted(() => {
  eventsStore.disconnect()
})

const connectionStatus = computed(() => {
  return eventsStore.connected ? 'Connected' : 'Disconnected'
})

const selectEvent = (id: string) => {
  selectedEventId.value = id
  activeTab.value = 'inspector'
}
</script>

<template>
  <div class="min-h-screen bg-gray-900 text-white">
    <!-- Header -->
    <header class="bg-gray-800 border-b border-gray-700 px-6 py-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <h1 class="text-2xl font-bold text-blue-400">CheckStream Demo</h1>
          <span class="text-sm text-gray-400">Interactive Guardrail Visualization</span>
        </div>
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-2">
            <div
              :class="[
                'w-2 h-2 rounded-full',
                eventsStore.connected ? 'bg-green-500 pulse-green' : 'bg-red-500 pulse-red'
              ]"
            ></div>
            <span class="text-sm text-gray-400">{{ connectionStatus }}</span>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="container mx-auto px-6 py-6">
      <!-- Top Row: Stats, Controls, Issues -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-6">
        <LiveStats />
        <TrafficControls />
        <IssueBreakdown />
      </div>

      <!-- Bottom Row: Event Log / Inspector -->
      <div class="bg-gray-800 rounded-lg">
        <!-- Tabs -->
        <div class="flex border-b border-gray-700">
          <button
            @click="activeTab = 'log'"
            :class="[
              'px-6 py-3 text-sm font-medium transition-colors',
              activeTab === 'log'
                ? 'text-blue-400 border-b-2 border-blue-400'
                : 'text-gray-400 hover:text-white'
            ]"
          >
            Event Log
          </button>
          <button
            @click="activeTab = 'inspector'"
            :class="[
              'px-6 py-3 text-sm font-medium transition-colors',
              activeTab === 'inspector'
                ? 'text-blue-400 border-b-2 border-blue-400'
                : 'text-gray-400 hover:text-white'
            ]"
          >
            Request Inspector
          </button>
        </div>

        <!-- Tab Content -->
        <div class="p-6">
          <EventLog
            v-if="activeTab === 'log'"
            @select="selectEvent"
          />
          <RequestInspector
            v-else
            :event-id="selectedEventId"
          />
        </div>
      </div>
    </main>
  </div>
</template>
