<script lang="ts">
import { Collapsible } from "bits-ui";
import MinusIcon from "lucide-svelte/icons/minus";
import PlusIcon from "lucide-svelte/icons/plus";


const { routes = [], currentSlug = "", ...restProps } = $props<{
    routes?: string[];
    currentSlug?: string;
}>();


interface Node {
    name: string;
    type: "file" | "directory";
    slug: string;
    children: Node[];
}

function flatten(map: Map<string, { node: Node; map: Map<string, any> }>): Node[] {

    return Array.from(map.values())
        .sort((a, b) => {
            if (a.node.type !== b.node.type) return a.node.type === "file" ? -1 : 1;
            return a.node.name.localeCompare(b.node.name);
        })
        .map((entry) => {
            entry.node.children = flatten(entry.map);
            return entry.node;
        });

}

function buildTree(routes: string[]): Node[] {
    const root = new Map<string, { node: Node; map: Map<string, any> }>()

    
    for (const route of routes) {
            const clean = route.replace(/^\.+\//, "").replace(/\.txt$/, "");
        const segments = clean.split("/").filter(Boolean).slice(2); // strips `.wiki/wiki/`
        let currentMap = root;

        for (let i = 0; i < segments.length; i++) {
            const seg = segments[i];
            const slug = segments.slice(0, i + 1).join("/");
            const isFile = i === segments.length - 1;
            
            if (!currentMap.has(seg)) {
                currentMap.set(seg, {
                node: {
                    name: seg,
                    type: isFile ? "file" : "directory",
                    slug,
                    children: [],
                },
                map: new Map(),
                });
            }

            if (!isFile) {
                currentMap = currentMap.get(seg)!.map;
            }
        }
    }   

    return flatten(root)

}



const tree = $derived(buildTree(routes));


function hasActive(node: Node): boolean {
    if (node.type === "file" && node.slug === currentSlug) return true;
    if (node.type === "directory") return node.children.some(hasActive);
    return false;
}

const openPaths = $derived.by(() => {
    const set = new Set<string>();
    function walk(nodes: Node[]) {
        for (const n of nodes) {
            if (n.type === "directory" && n.children.some(hasActive)) {
                set.add(n.slug);
                walk(n.children);
            }
        }
    }
    walk(tree);
    return set;
});

function isOpen(slug: string) {
    return openPaths.has(slug);
}
</script>


<aside class="sidebar-nav">
    <nav>
        {#each tree as node (node.slug)}
            {#if node.type === "directory"}
                <Collapsible.Root open={isOpen(node.slug)} class="group/collapsible">
                    <Collapsible.Trigger class="sidebar-trigger">
                        {node.name}
                        <PlusIcon class="ms-auto group-data-[state=open]/collapsible:hidden" />
                        <MinusIcon class="ms-auto group-data-[state=closed]/collapsible:hidden" />
                    </Collapsible.Trigger>
                    <Collapsible.Content>
                        <ul class="sidebar-sub">
                            {#each node.children as child (child.slug)}
                                {#if child.type === "directory"}
                                    <li>
                                        <Collapsible.Root open={isOpen(child.slug)} class="group/collapsible">
                                            <Collapsible.Trigger class="sidebar-trigger sidebar-sub-trigger">
                                                {child.name}
                                                <PlusIcon class="ms-auto group-data-[state=open]/collapsible:hidden" />
                                                <MinusIcon class="ms-auto group-data-[state=closed]/collapsible:hidden" />
                                            </Collapsible.Trigger>
                                            <Collapsible.Content>
                                                <ul class="sidebar-sub">
                                                    {#each child.children as grandchild (grandchild.slug)}
                                                        <li>
                                                            <a href={"/wiki/" + grandchild.slug} class="sidebar-link" class:active={grandchild.slug === currentSlug}>{grandchild.name}</a>
                                                        </li>
                                                    {/each}
                                                </ul>
                                            </Collapsible.Content>
                                        </Collapsible.Root>
                                    </li>
                                {:else}
                                    <li>
                                        <a href={"/wiki/" + child.slug} class="sidebar-link" class:active={child.slug === currentSlug}>{child.name}</a>
                                    </li>
                                {/if}
                            {/each}
                        </ul>
                    </Collapsible.Content>
                </Collapsible.Root>
            {:else}
                <a href={"/wiki/" + node.slug} class="sidebar-link" class:active={node.slug === currentSlug}>{node.name}</a>
            {/if}
        {/each}
    </nav>
</aside>


<style>
    .sidebar-nav {
        padding: 0.5rem 0;
    }
    .sidebar-nav nav {
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
    }
    .sidebar-trigger {
        display: flex;
        align-items: center;
        width: 100%;
        padding: 0.5rem 0.75rem;
        font-size: 0.875rem;
        cursor: pointer;
        border: none;
        background: none;
        text-align: left;
        border-radius: 0.375rem;
    }
    .sidebar-trigger:hover {
        background: var(--color-muted, #f5f5f5);
    }
    .sidebar-sub-trigger {
        padding-left: 1.5rem;
    }
    .sidebar-sub {
        list-style: none;
        margin: 0;
        padding: 0;
    }
    .sidebar-link {
        display: block;
        padding: 0.375rem 0.75rem;
        font-size: 0.875rem;
        text-decoration: none;
        color: inherit;
        border-radius: 0.375rem;
    }
    .sidebar-link:hover {
        background: var(--color-muted, #f5f5f5);
    }
    .sidebar-link.active {
        background: var(--color-primary, #000);
        color: var(--color-primary-foreground, #fff);
    }
</style>
