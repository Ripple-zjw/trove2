import { writable } from 'svelte/store';
import type { Tool } from './types';

export const tools = writable<Tool[]>([]);
export const searchQuery = writable('');
export const selectedCategory = writable('');
