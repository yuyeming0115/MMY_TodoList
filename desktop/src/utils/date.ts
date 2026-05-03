// 日期计算工具

export function daysLeft(dueDate: number | undefined): string | null {
  if (!dueDate) return null;

  const now = new Date();
  const due = new Date(dueDate);
  const diff = Math.floor((due.getTime() - new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()) / (1000 * 60 * 60 * 24));

  if (diff < 0) return `已逾期 ${-diff} 天`;
  if (diff === 0) return '今天到期';
  return `还有 ${diff} 天`;
}

export function formatDate(timestamp: number | undefined): string {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  return `${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()}`;
}

export function isOverdue(dueDate: number | undefined): boolean {
  if (!dueDate) return false;
  const now = new Date();
  const due = new Date(dueDate);
  return due < new Date(now.getFullYear(), now.getMonth(), now.getDate());
}

export function isDueToday(dueDate: number | undefined): boolean {
  if (!dueDate) return false;
  const now = new Date();
  const due = new Date(dueDate);
  return due.getFullYear() === now.getFullYear() &&
         due.getMonth() === now.getMonth() &&
         due.getDate() === now.getDate();
}