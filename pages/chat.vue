<script setup lang="ts">
import type { ChatResponse } from '~/shared/types/api'

interface Message {
  role: 'user' | 'assistant'
  text: string
  sources?: ChatResponse['sources']
}

const api = useApi()
const input = ref('')
const pending = ref(false)
const messages = ref<Message[]>([])

async function send() {
  const text = input.value.trim()
  if (!text || pending.value) return

  messages.value.push({ role: 'user', text })
  input.value = ''
  pending.value = true

  try {
    const res = await api.chat({ message: text })
    messages.value.push({ role: 'assistant', text: res.answer, sources: res.sources })
  } catch {
    messages.value.push({
      role: 'assistant',
      text: 'Sorry — something went wrong talking to the chat API.',
    })
  } finally {
    pending.value = false
  }
}
</script>

<template>
  <NuxtLayout>
    <template #title>Chat assistant</template>

    <UCard class="max-w-3xl mx-auto flex flex-col" :ui="{ body: { base: 'flex-1' } }">
      <div class="space-y-4 min-h-[50vh]">
        <p v-if="!messages.length" class="text-sm text-gray-400 text-center py-12">
          Ask a question to try the dedicated <code>/api/chat</code> endpoint.
        </p>

        <div
          v-for="(msg, i) in messages"
          :key="i"
          class="flex"
          :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
        >
          <div
            class="max-w-[80%] rounded-lg px-4 py-2 text-sm"
            :class="
              msg.role === 'user'
                ? 'bg-primary-500 text-white'
                : 'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100'
            "
          >
            <p class="whitespace-pre-wrap">{{ msg.text }}</p>
            <ul v-if="msg.sources?.length" class="mt-2 text-xs opacity-80 list-disc pl-4">
              <li v-for="(s, si) in msg.sources" :key="si">{{ s.filename }} — {{ s.heading }}</li>
            </ul>
          </div>
        </div>
      </div>

      <template #footer>
        <form class="flex gap-2" @submit.prevent="send">
          <UInput
            v-model="input"
            placeholder="Type a message…"
            class="flex-1"
            :disabled="pending"
            autofocus
          />
          <UButton type="submit" :loading="pending" :disabled="!input.trim()">Send</UButton>
        </form>
      </template>
    </UCard>
  </NuxtLayout>
</template>
