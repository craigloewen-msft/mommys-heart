<script setup lang="ts">
import type { Contact } from '~/shared/types/api'

const api = useApi()
const { data: contacts } = await useAsyncData('contacts-list', () => api.listContacts())

const statusColor: Record<Contact['status'], string> = {
  active: 'green',
  lead: 'blue',
  inactive: 'gray',
}
</script>

<template>
  <NuxtLayout>
    <template #title>Contacts</template>

    <UCard :ui="{ body: { padding: '' } }">
      <table class="w-full text-sm">
        <thead class="text-left text-gray-500 border-b border-gray-200 dark:border-gray-800">
          <tr>
            <th class="px-4 py-3 font-medium">Name</th>
            <th class="px-4 py-3 font-medium">Company</th>
            <th class="px-4 py-3 font-medium">Email</th>
            <th class="px-4 py-3 font-medium">Status</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="contact in contacts"
            :key="contact.id"
            class="border-b border-gray-100 dark:border-gray-800 last:border-0 hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer"
            @click="navigateTo(`/contacts/${contact.id}`)"
          >
            <td class="px-4 py-3 font-medium text-gray-900 dark:text-white">{{ contact.name }}</td>
            <td class="px-4 py-3 text-gray-600 dark:text-gray-300">{{ contact.company }}</td>
            <td class="px-4 py-3 text-gray-600 dark:text-gray-300">{{ contact.email }}</td>
            <td class="px-4 py-3">
              <UBadge :color="statusColor[contact.status]" variant="subtle" class="capitalize">
                {{ contact.status }}
              </UBadge>
            </td>
          </tr>
        </tbody>
      </table>
    </UCard>
  </NuxtLayout>
</template>
