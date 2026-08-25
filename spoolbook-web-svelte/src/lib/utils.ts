import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// Svelte's bind:value on <input type="number"> yields an actual number (or '' when empty) at
// runtime regardless of a $state('') declaration's inferred string type -- so a form field bound
// this way must be typed number | string and read back with this, not v.trim().
export function numOrNull(v: number | string): number | null {
	if (v === '') return null;
	return typeof v === 'number' ? v : Number(v);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
