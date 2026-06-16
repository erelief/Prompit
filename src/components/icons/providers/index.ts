import type { Component } from 'vue'
import OpenAI from './OpenAI.vue'
import DeepSeek from './DeepSeek.vue'
import Moonshot from './Moonshot.vue'
import Zhipu from './Zhipu.vue'
import Minimax from './Minimax.vue'
import LMStudio from './LMStudio.vue'
import Anthropic from './Anthropic.vue'
import Gemini from './Gemini.vue'
import Grok from './Grok.vue'
import Qwen from './Qwen.vue'
import Bailian from './Bailian.vue'
import xAI from './xAI.vue'
import Volcengine from './Volcengine.vue'
import MIMO from './MIMO.vue'
import Step from './Step.vue'

export const PROVIDER_ICONS: Record<string, Component> = {
  openai: OpenAI,
  deepseek: DeepSeek,
  moonshot: Moonshot,
  zhipu: Zhipu,
  minimax: Minimax,
  lmstudio: LMStudio,
  anthropic: Anthropic,
  gemini: Gemini,
  grok: Grok,
  qwen: Qwen,
  bailian: Bailian,
  xai: xAI,
  volcengine: Volcengine,
  mimo: MIMO,
  step: Step,
}
