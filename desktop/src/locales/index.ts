import zh from './zh';
import en from './en';

const locales = { zh, en };

export type LocaleKey = 'zh' | 'en';
export type LocaleStrings = typeof zh;

// 获取翻译字符串
// 支持嵌套路径如 'sidebar.tasks'
// 支持插值如 t('expiry.expiresIn', { time: '3天' }) => '3天后过期'
export function t(key: string, params?: Record<string, string | number>, locale: LocaleKey = 'zh'): string {
  const strings = locales[locale];

  // 解析嵌套路径
  const parts = key.split('.');
  let result: any = strings;

  for (const part of parts) {
    if (result && typeof result === 'object' && part in result) {
      result = result[part];
    } else {
      // 找不到翻译，返回 key
      console.warn(`Translation not found: ${key}`);
      return key;
    }
  }

  if (typeof result !== 'string') {
    console.warn(`Translation is not a string: ${key}`);
    return key;
  }

  // 处理插值
  if (params) {
    return result.replace(/\{(\w+)\}/g, (_, name) => {
      return params[name]?.toString() ?? `{${name}}`;
    });
  }

  return result;
}

// 获取整个 locale 对象
export function getLocale(locale: LocaleKey): LocaleStrings {
  return locales[locale];
}

// 导出 locales 供直接访问
export { zh, en };