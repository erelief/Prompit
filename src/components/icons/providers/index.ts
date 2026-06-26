import type { Component } from 'vue'
import OpenAI from './OpenAI.vue'
import OpenCode from './OpenCode.vue'
import DeepSeek from './DeepSeek.vue'
import Moonshot from './Moonshot.vue'
import Kimi from './Kimi.vue'
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
import MiMo from './MiMo.vue'
import Step from './Step.vue'
import Agnes from './Agnes.vue'
import OpenRouter from './OpenRouter.vue'
import AnySearch from './AnySearch.vue'
import Brave from './Brave.vue'
import Tavily from './Tavily.vue'
import Exa from './Exa.vue'
import SiliconFlow from './SiliconFlow.vue'
import SenseNova from './SenseNova.vue'

export const PROVIDER_ICONS: Record<string, Component> = {
  openai: OpenAI,
  opencode: OpenCode,
  deepseek: DeepSeek,
  moonshot: Moonshot,
  kimi: Kimi,
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
  mimo: MiMo,
  step: Step,
  agnes: Agnes,
  openrouter: OpenRouter,
  anysearch: AnySearch,
  brave: Brave,
  tavily: Tavily,
  exa: Exa,
  siliconflow: SiliconFlow,
  sensenova: SenseNova,
}
