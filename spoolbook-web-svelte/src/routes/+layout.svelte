<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/spoolbook.ico';
	import spoolbookIcon from '$lib/assets/spoolbook-icon.png';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { getVersion, me, logout, setupStatus } from '$lib/api/client';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
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
		{ href: '/spools', label: 'Spools' }
	];
	const workflowSubItems = [
		{ href: '/profiles', label: 'Profiles' },
		{ href: '/prints', label: 'Prints' },
		{ href: '/printers', label: 'Printers' },
		{ href: '/projects', label: 'Projects' }
	];
	const bottomItems = [{ href: '/settings', label: 'Settings', icon: SettingsIcon }];

	function isActive(href: string) {
		return href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
	}

	let filamentsOpen = $state(filamentsSubItems.some((item) => isActive(item.href)));
	let workflowOpen = $state(workflowSubItems.some((item) => isActive(item.href)));
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
				<div class="flex items-center gap-2 px-2 py-1.5">
					<img src={spoolbookIcon} alt="Spoolbook" class="size-8 rounded-lg" />
					<div class="flex flex-col leading-tight">
						<span class="text-sm font-semibold">Spoolbook</span>
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
									<Sidebar.MenuButton>
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
											<Sidebar.MenuButton {...props}>
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
													<Sidebar.MenuSubButton href={item.href}>{item.label}</Sidebar.MenuSubButton>
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
											<Sidebar.MenuButton {...props}>
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
													<Sidebar.MenuSubButton href={item.href}>{item.label}</Sidebar.MenuSubButton>
												</Sidebar.MenuSubItem>
											{/each}
										</Sidebar.MenuSub>
									</Collapsible.Content>
								</Sidebar.MenuItem>
							</Collapsible.Root>

							{#each bottomItems as item (item.href)}
								<Sidebar.MenuItem>
									<Sidebar.MenuButton>
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
					</Sidebar.GroupContent>
				</Sidebar.Group>
			</Sidebar.Content>
			<Sidebar.Footer>
				{#if authenticated}
					<NavUser {username} onSignOut={signOut} />
				{:else}
					<Sidebar.Menu>
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href="/login" {...props}>Sign in</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					</Sidebar.Menu>
				{/if}
			</Sidebar.Footer>
		</Sidebar.Root>

		<Sidebar.Inset>
			<header class="flex h-12 shrink-0 items-center gap-2 border-b px-4">
				<Sidebar.Trigger />
			</header>
			{@render children()}
		</Sidebar.Inset>
	</Sidebar.Provider>
{/if}
