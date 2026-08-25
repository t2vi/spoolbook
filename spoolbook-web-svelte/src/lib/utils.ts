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

// App-wide date format: "MMM DD, YYYY" (e.g. "Aug 25, 2026"). Table cells use formatDate alone
// (no time, per the tables convention); other date+time displays append formatTime's output.
export function formatDate(iso: string): string {
	return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: '2-digit', year: 'numeric' });
}

export function formatDateTime(iso: string): string {
	const time = new Date(iso).toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
	return `${formatDate(iso)}, ${time}`;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
