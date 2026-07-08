<script setup lang="ts">
const api = useApi()
const { data: contacts } = await useAsyncData('contacts', () => api.listContacts())
const { data: version } = await useAsyncData('version', () => api.version())

const stats = computed(() => {
  const list = contacts.value ?? []
  return [
    { label: 'Total contacts', value: list.length, icon: 'i-heroicons-users' },
    {
      label: 'Active',
      value: list.filter((c) => c.status === 'active').length,
      icon: 'i-heroicons-check-badge',
    },
    {
      label: 'Leads',
      value: list.filter((c) => c.status === 'lead').length,
      icon: 'i-heroicons-sparkles',
    },
    {
      label: 'Inactive',
      value: list.filter((c) => c.status === 'inactive').length,
      icon: 'i-heroicons-moon',
    },
  ]
})
</script>

<template>
  <NuxtLayout>
    <template #title>Dashboard</template>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      <UCard v-for="stat in stats" :key="stat.label">
        <div class="flex items-center gap-4">
          <div class="p-3 rounded-lg bg-primary-50 dark:bg-primary-900/30">
            <UIcon :name="stat.icon" class="text-2xl text-primary-500" />
          </div>
          <div>
            <p class="text-2xl font-semibold text-gray-900 dark:text-white">{{ stat.value }}</p>
            <p class="text-sm text-gray-500">{{ stat.label }}</p>
          </div>
        </div>
      </UCard>
    </div>

    <UCard class="mt-6">
      <template #header>
        <div class="flex items-center justify-between">
          <h2 class="font-semibold text-gray-900 dark:text-white">Welcome</h2>
          <UBadge v-if="version" color="gray" variant="subtle">API v{{ version.version }}</UBadge>
        </div>
      </template>
      <p class="text-sm text-gray-600 dark:text-gray-300">
        This is the scaffolded Mommy's Heart CRM. Use the sidebar to browse
        <NuxtLink to="/contacts" class="text-primary-500 hover:underline">Contacts</NuxtLink>
        or talk to the
        <NuxtLink to="/chat" class="text-primary-500 hover:underline">assistant</NuxtLink>. The
        dashboard, data, and chat are placeholders wired to the dedicated API — ready to augment.
      </p>
    </UCard>
  </NuxtLayout>
</template>
