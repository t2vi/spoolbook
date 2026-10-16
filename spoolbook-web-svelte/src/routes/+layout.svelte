<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/spoolbook.ico';
	import spoolbookIcon from '$lib/assets/spoolbook-icon.png';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { getVersion, me, logout, setupStatus } from '$lib/api/client';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import * as Breadcrumb from '$lib/components/ui/breadcrumb/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import HouseIcon from '@lucide/svelte/icons/house';
	import MoonIcon from '@lucide/svelte/icons/moon';
	import SunIcon from '@lucide/svelte/icons/sun';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import CylinderIcon from '@lucide/svelte/icons/cylinder';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import PrinterIcon from '@lucide/svelte/icons/printer';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import NavUser from '$lib/components/nav-user.svelte';

	let { children } = $props();

	let authenticated = $state(false);
	// Nothing renders until the session is known, so a signed-out visitor never sees app data.
	let checked = $state(false);
	let username = $state('');
	let version = $state('');
	let isAuthPage = $derived(page.url.pathname === '/login' || page.url.pathname === '/setup');

	$effect(() => {
		getVersion().then((r) => (version = r.version));
	});

	// Re-checked on every navigation (e.g. right after signing in). No account yet → /setup;
	// signed out → /login. The API refuses every read without a session too; this is the UI half.
	$effect(() => {
		const path = page.url.pathname;
		Promise.all([me(), setupStatus()]).then(([r, s]) => {
			authenticated = r.authenticated;
			username = r.username ?? '';
			checked = true;
			if (s.needsSetup && path !== '/setup') goto('/setup');
			else if (!s.needsSetup && !r.authenticated && path !== '/login') goto('/login');
		});
	});

	async function signOut() {
		await logout();
		authenticated = false;
		goto('/login');
	}

	const topItems = [{ href: '/', label: 'Dashboard', icon: LayoutDashboardIcon }];
	const filamentsSubItems = [
		{ href: '/filaments', label: 'List' },
		{ href: '/spools', label: 'Spools' },
		{ href: '/filaments/colors', label: 'Colors' }
	];
	const workflowSubItems = [
		{ href: '/profiles', label: 'Profiles' },
		{ href: '/prints', label: 'Prints' },
		{ href: '/printers', label: 'Printers' },
		{ href: '/projects', label: 'Projects' }
	];
	const bottomItems = [{ href: '/settings', label: 'Settings', icon: SettingsIcon }];

	// The longest matching link wins, so /filaments/colors highlights Colors, not Filaments too.
	const navItems = [...topItems, ...filamentsSubItems, ...workflowSubItems, ...bottomItems];
	const navHrefs = navItems.map((i) => i.href);
	function isActive(href: string) {
		const path = page.url.pathname;
		const matches = (h: string) => (h === '/' ? path === '/' : path === h || path.startsWith(`${h}/`));
		return matches(href) && !navHrefs.some((h) => h.length > href.length && matches(h));
	}

	let filamentsOpen = $state(filamentsSubItems.some((item) => isActive(item.href)));
	let workflowOpen = $state(workflowSubItems.some((item) => isActive(item.href)));

	// Home / group / page / New|Edit|Details, named as the sidebar names them (002/FR-009).
	const groups = [
		{ label: 'Filaments', items: filamentsSubItems },
		{ label: 'Print workflow', items: workflowSubItems }
	];
	let crumbs = $derived.by(() => {
		const item = navItems.find((i) => isActive(i.href));
		if (!item) return null;
		const rest = page.url.pathname.slice(item.href.length);
		const sub = rest.startsWith('/new') ? 'New' : rest.startsWith('/edit') ? 'Edit' : rest.length > 1 ? 'Details' : null;
		return { group: groups.find((g) => g.items.includes(item))?.label, item, sub };
	});

	// app.html applies the remembered choice (or the system setting) before first paint.
	let dark = $state(document.documentElement.classList.contains('dark'));
	function toggleTheme() {
		dark = !dark;
		document.documentElement.classList.toggle('dark', dark);
		try {
			localStorage.setItem('theme', dark ? 'dark' : 'light');
		} catch {}
	}
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

{#if checked && !authenticated && isAuthPage}
	<main class="flex min-h-svh flex-col items-center justify-center gap-6 p-4">
		{@render children()}
		<p class="text-xs text-muted-foreground">
			Spoolbook{version ? ` v${version}` : ''} · <a href="https://github.com/t2vi/spoolbook" target="_blank" rel="noopener" class="underline-offset-2 hover:underline">Source</a>
		</p>
	</main>
{:else if checked && authenticated}
	<Sidebar.Provider>
		<Sidebar.Root>
			<Sidebar.Header>
				<div class="flex items-center gap-2.5 px-2 py-3">
					<img src={spoolbookIcon} alt="Spoolbook" class="size-9 rounded-xl" />
					<div class="flex flex-col leading-tight">
						<span class="text-base font-bold text-primary">Spoolbook</span>
						<span class="text-xs text-sidebar-foreground/70">
							{version ? `v${version} · ` : ''}<a href="https://github.com/t2vi/spoolbook" target="_blank" rel="noopener" class="underline-offset-2 hover:underline">Source</a>
						</span>
					</div>
				</div>
			</Sidebar.Header>
			<Sidebar.Content>
				<Sidebar.Group>
					<Sidebar.GroupContent>
						<Sidebar.Menu>
							{#each topItems as item (item.href)}
								<Sidebar.MenuItem>
									<Sidebar.MenuButton isActive={isActive(item.href)} class="h-10">
										{#snippet child({ props })}
											<a href={item.href} {...props}>
												<item.icon />
												{item.label}
											</a>
										{/snippet}
									</Sidebar.MenuButton>
								</Sidebar.MenuItem>
							{/each}

							<Collapsible.Root bind:open={filamentsOpen} class="group/collapsible">
								<Sidebar.MenuItem>
									<Collapsible.Trigger>
										{#snippet child({ props })}
											<Sidebar.MenuButton {...props} class="h-10">
												<CylinderIcon />
												Filaments
												<ChevronRightIcon class="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-90" />
											</Sidebar.MenuButton>
										{/snippet}
									</Collapsible.Trigger>
									<Collapsible.Content>
										<Sidebar.MenuSub>
											{#each filamentsSubItems as item (item.href)}
												<Sidebar.MenuSubItem>
													<Sidebar.MenuSubButton href={item.href} isActive={isActive(item.href)} class="h-9">{item.label}</Sidebar.MenuSubButton>
												</Sidebar.MenuSubItem>
											{/each}
										</Sidebar.MenuSub>
									</Collapsible.Content>
								</Sidebar.MenuItem>
							</Collapsible.Root>

							<Collapsible.Root bind:open={workflowOpen} class="group/collapsible">
								<Sidebar.MenuItem>
									<Collapsible.Trigger>
										{#snippet child({ props })}
											<Sidebar.MenuButton {...props} class="h-10">
												<PrinterIcon />
												Print workflow
												<ChevronRightIcon class="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-90" />
											</Sidebar.MenuButton>
										{/snippet}
									</Collapsible.Trigger>
									<Collapsible.Content>
										<Sidebar.MenuSub>
											{#each workflowSubItems as item (item.href)}
												<Sidebar.MenuSubItem>
													<Sidebar.MenuSubButton href={item.href} isActive={isActive(item.href)} class="h-9">{item.label}</Sidebar.MenuSubButton>
												</Sidebar.MenuSubItem>
											{/each}
										</Sidebar.MenuSub>
									</Collapsible.Content>
								</Sidebar.MenuItem>
							</Collapsible.Root>

						</Sidebar.Menu>
					</Sidebar.GroupContent>
				</Sidebar.Group>
			</Sidebar.Content>
			<Sidebar.Footer>
				<div class="rounded-xl bg-muted p-1">
					<Sidebar.Menu>
						{#each bottomItems as item (item.href)}
							<Sidebar.MenuItem>
								<Sidebar.MenuButton isActive={isActive(item.href)} class="h-10">
									{#snippet child({ props })}
										<a href={item.href} {...props}>
											<item.icon />
											{item.label}
										</a>
									{/snippet}
								</Sidebar.MenuButton>
							</Sidebar.MenuItem>
						{/each}
					</Sidebar.Menu>
					<NavUser {username} onSignOut={signOut} />
				</div>
			</Sidebar.Footer>
		</Sidebar.Root>

		<Sidebar.Inset>
			<header class="flex h-16 shrink-0 items-center gap-3 px-4 md:px-6">
				<Sidebar.Trigger class="-ml-1" />
				{#if crumbs}
					<Breadcrumb.Root>
						<Breadcrumb.List>
							<Breadcrumb.Item>
								<Breadcrumb.Link href="/"><HouseIcon class="size-4" /><span class="sr-only">Home</span></Breadcrumb.Link>
							</Breadcrumb.Item>
							{#if crumbs.group}
								<Breadcrumb.Separator />
								<Breadcrumb.Item>{crumbs.group}</Breadcrumb.Item>
							{/if}
							<Breadcrumb.Separator />
							<Breadcrumb.Item>
								{#if crumbs.sub}
									<Breadcrumb.Link href={crumbs.item.href}>{crumbs.item.label}</Breadcrumb.Link>
								{:else}
									<Breadcrumb.Page>{crumbs.item.label}</Breadcrumb.Page>
								{/if}
							</Breadcrumb.Item>
							{#if crumbs.sub}
								<Breadcrumb.Separator />
								<Breadcrumb.Item><Breadcrumb.Page>{crumbs.sub}</Breadcrumb.Page></Breadcrumb.Item>
							{/if}
						</Breadcrumb.List>
					</Breadcrumb.Root>
				{/if}
				<div class="ml-auto flex items-center rounded-full border bg-card p-1 shadow-xs">
					<Button variant="ghost" size="icon" class="rounded-full" onclick={toggleTheme} aria-label={dark ? 'Switch to light mode' : 'Switch to dark mode'}>
						{#if dark}<SunIcon />{:else}<MoonIcon />{/if}
					</Button>
				</div>
			</header>
			{@render children()}
		</Sidebar.Inset>
	</Sidebar.Provider>
{/if}
