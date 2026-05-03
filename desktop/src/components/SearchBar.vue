<script setup lang="ts">
import { ref } from 'vue';
import { NInput, NIcon } from 'naive-ui';
import { Search as SearchIcon } from '@vicons/ionicons5';
import { useTaskStore } from '../stores/taskStore';

const store = useTaskStore();
const inputValue = ref('');

function onInput(value: string) {
  store.searchQuery = value;
}

function clear() {
  inputValue.value = '';
  store.searchQuery = '';
}
</script>

<template>
  <NInput
    v-model:value="inputValue"
    placeholder="搜索任务..."
    clearable
    size="small"
    @update:value="onInput"
    @clear="clear"
  >
    <template #prefix>
      <NIcon :component="SearchIcon" />
    </template>
  </NInput>
</template>