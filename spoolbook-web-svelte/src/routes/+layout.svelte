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
	let username = $state('');
	let version = $state('');

	$effect(() => {
		me().then((r) => {
			authenticated = r.authenticated;
			username = r.username ?? '';
		});
		getVersion().then((r) => (version = r.version));
	});

	// No admin account exists yet -- every route redirects to the wizard until one does, except
	// /setup itself (nothing to redirect to/from there).
	$effect(() => {
		if (page.url.pathname === '/setup') return;
		setupStatus().then((s) => {
			if (s.needsSetup) goto('/setup');
		});
	});

	async function signOut() {
		await logout();
		authenticated = false;
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

<Sidebar.Provider>
	<Sidebar.Root>
		<Sidebar.Header>
			<div class="flex items-center gap-2 px-2 py-1.5">
				<img src={spoolbookIcon} alt="Spoolbook" class="size-8 rounded-lg" />
				<div class="flex flex-col leading-tight">
					<span class="text-sm font-semibold">Spoolbook</span>
					<span class="text-xs text-sidebar-foreground/70">{version ? `v${version}` : ''}</span>
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
