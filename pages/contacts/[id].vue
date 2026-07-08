<script setup lang="ts">
const route = useRoute()
const api = useApi()
const id = route.params.id as string

const { data: contact, error } = await useAsyncData(`contact-${id}`, () => api.getContact(id))

const statusColor = {
  active: 'green',
  lead: 'blue',
  inactive: 'gray',
} as const
</script>

<template>
  <NuxtLayout>
    <template #title>Contact</template>

    <UButton
      to="/contacts"
      icon="i-heroicons-arrow-left"
      variant="link"
      color="gray"
      class="mb-4 px-0"
    >
      Back to contacts
    </UButton>

    <UCard v-if="contact">
      <template #header>
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-lg font-semibold text-gray-900 dark:text-white">{{ contact.name }}</h2>
            <p class="text-sm text-gray-500">{{ contact.company }}</p>
          </div>
          <UBadge :color="statusColor[contact.status]" variant="subtle" class="capitalize">
            {{ contact.status }}
          </UBadge>
        </div>
      </template>

      <dl class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
        <div>
          <dt class="text-gray-500">Email</dt>
          <dd class="text-gray-900 dark:text-white">{{ contact.email }}</dd>
        </div>
        <div>
          <dt class="text-gray-500">Phone</dt>
          <dd class="text-gray-900 dark:text-white">{{ contact.phone }}</dd>
        </div>
        <div class="sm:col-span-2">
          <dt class="text-gray-500">Notes</dt>
          <dd class="text-gray-900 dark:text-white">{{ contact.notes }}</dd>
        </div>
        <div>
          <dt class="text-gray-500">Created</dt>
          <dd class="text-gray-900 dark:text-white">
            {{ new Date(contact.createdAt).toLocaleDateString() }}
          </dd>
        </div>
      </dl>
    </UCard>

    <UCard v-else-if="error">
      <p class="text-sm text-red-500">Contact not found.</p>
    </UCard>
  </NuxtLayout>
</template>
