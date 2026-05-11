import { computed } from 'vue';
import { t, LocaleKey } from '../locales';
import { useSettingsStore } from '../stores/settingsStore';

/**
 * 翻译 composable
 * 自动响应语言设置变化
 */
export function useI18n() {
  const settingsStore = useSettingsStore();

  // 当前语言（安全访问，防止 settings 未初始化）
  const locale = computed<LocaleKey>(() => {
    const lang = settingsStore.settings?.language;
    return lang === 'en' ? 'en' : 'zh';
  });

  // 翻译函数（响应式）
  const tt = (key: string, params?: Record<string, string | number>): string => {
    return t(key, params, locale.value);
  };

  // 是否为英文
  const isEnglish = computed(() => locale.value === 'en');

  return {
    locale,
    isEnglish,
    t: tt,
  };
}