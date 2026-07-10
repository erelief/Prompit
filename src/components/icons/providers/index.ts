import type { Component } from "vue";
import Agnes from "./Agnes.vue";
import Anthropic from "./Anthropic.vue";
import AnySearch from "./AnySearch.vue";
import Baidu from "./Baidu.vue";
import Bailian from "./Bailian.vue";
import Brave from "./Brave.vue";
import DeepSeek from "./DeepSeek.vue";
import Exa from "./Exa.vue";
import Gemini from "./Gemini.vue";
import Grok from "./Grok.vue";
import Kimi from "./Kimi.vue";
import LMStudio from "./LMStudio.vue";
import Meta from "./Meta.vue";
import MiMo from "./MIMO.vue";
import Minimax from "./Minimax.vue";
import Moonshot from "./Moonshot.vue";
import OpenCode from "./OpenCode.vue";
import OpenAI from "./OpenAI.vue";
import OpenRouter from "./OpenRouter.vue";
import Qwen from "./Qwen.vue";
import SenseNova from "./SenseNova.vue";
import SiliconFlow from "./SiliconFlow.vue";
import Step from "./Step.vue";
import StreamLake from "./StreamLake.vue";
import Tavily from "./Tavily.vue";
import TencentCloud from "./TencentCloud.vue";
import Volcengine from "./Volcengine.vue";
import xAI from "./xAI.vue";
import Zhipu from "./Zhipu.vue";

export const PROVIDER_ICONS: Record<string, Component> = {
  agnes: Agnes,
  anthropic: Anthropic,
  anysearch: AnySearch,
  baidu: Baidu,
  bailian: Bailian,
  brave: Brave,
  deepseek: DeepSeek,
  exa: Exa,
  gemini: Gemini,
  grok: Grok,
  kimi: Kimi,
  lmstudio: LMStudio,
  meta: Meta,
  mimo: MiMo,
  minimax: Minimax,
  moonshot: Moonshot,
  opencode: OpenCode,
  openai: OpenAI,
  openrouter: OpenRouter,
  qwen: Qwen,
  sensenova: SenseNova,
  siliconflow: SiliconFlow,
  step: Step,
  streamlake: StreamLake,
  tavily: Tavily,
  tencentcloud: TencentCloud,
  volcengine: Volcengine,
  xai: xAI,
  zhipu: Zhipu,
};
