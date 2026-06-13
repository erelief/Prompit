import type { Component } from 'vue'
import OpenAI from './OpenAI.vue'
import DeepSeek from './DeepSeek.vue'
import Kimi from './Kimi.vue'
import Zhipu from './Zhipu.vue'
import Minimax from './Minimax.vue'
import Ollama from './Ollama.vue'

export const PROVIDER_ICONS: Record<string, Component> = {
  openai: OpenAI,
  deepseek: DeepSeek,
  kimi: Kimi,
  zhipu: Zhipu,
  minimax: Minimax,
  ollama: Ollama,
}
