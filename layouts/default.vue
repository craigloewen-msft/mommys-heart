<script setup lang="ts">
const nav = [
  { label: 'Dashboard', icon: 'i-heroicons-home', to: '/' },
  { label: 'Contacts', icon: 'i-heroicons-users', to: '/contacts' },
  { label: 'Chat', icon: 'i-heroicons-chat-bubble-left-right', to: '/chat' },
]

const route = useRoute()
const isActive = (to: string) =>
  to === '/' ? route.path === '/' : route.path.startsWith(to)
</script>

<template>
  <div class="min-h-screen flex bg-gray-50 dark:bg-gray-900">
    <!-- Sidebar -->
    <aside
      class="w-60 shrink-0 border-r border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-950 flex flex-col"
    >
      <div class="h-16 flex items-center gap-2 px-4 border-b border-gray-200 dark:border-gray-800">
        <UIcon name="i-heroicons-heart" class="text-primary-500 text-2xl" />
        <span class="font-semibold text-gray-900 dark:text-white">Mommy's Heart CRM</span>
      </div>
      <nav class="flex-1 p-3 space-y-1">
        <NuxtLink
          v-for="item in nav"
          :key="item.to"
          :to="item.to"
          class="flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors"
          :class="
            isActive(item.to)
              ? 'bg-primary-50 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300'
              : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
          "
        >
          <UIcon :name="item.icon" class="text-lg" />
          {{ item.label }}
        </NuxtLink>
      </nav>
      <div class="p-3 text-xs text-gray-400 border-t border-gray-200 dark:border-gray-800">
        v{{ useRuntimeConfig().public.appVersion }}
      </div>
    </aside>

    <!-- Main -->
    <div class="flex-1 flex flex-col min-w-0">
      <header
        class="h-16 shrink-0 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-950 flex items-center justify-between px-6"
      >
        <h1 class="text-lg font-semibold text-gray-900 dark:text-white">
          <slot name="title">Mommy's Heart CRM</slot>
        </h1>
        <UButton
          icon="i-heroicons-arrow-top-right-on-square"
          variant="ghost"
          color="gray"
          to="/chat"
        >
          Ask the assistant
        </UButton>
      </header>
      <main class="flex-1 overflow-auto p-6">
        <slot />
      </main>
    </div>
  </div>
</template>
