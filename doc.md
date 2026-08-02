# Documentation for IAC
IAC is Mast's ACL protocol that determines the visibility and permissible actions of certain wikipages and files to different end users. In IAC, permissions are tied to namespaces and pages, and a user's permission for a given page is resolved by finding the closest, most specific matching rule.


Before we continue, as IAC and ACLs in general are highly complex, we recommend you take a minute to review the key terminologies and language that will be used throughout this documentation.
- **Scope**: An word that refers to: wikipages, namespaces, and media files.
- **Grant** : See the Permission Values section

## Permission values (PV)
Permission values, or **PV**, consist of single-charactar values that includes N, R, U, C, and D, which stand for None, Read, Update, Create, and Delete. These values apply to scopes and govern which user receives which permission. The PV the user ultimately ends up receiving that governs their permission is called the Effective Permission Value, or **EPV**.

PV is hierarchical: which means any user with a certain PV will inherit access for all below it. D is the highest, most powerful PV, followed by C, then U, then R, then N. This means that a user with D as their PV will also have the permission to create, update, and read that specific scope.

N is the most restrictive PV: it does not permit users with this PV to even read the page. It is up to the administrator to determine whether to show this page as a 404 (hides the existence of the page/namespace) or as a block (merely shows the page is blocked - i.e. end users are able to know the existence of the page/namespace).

## Scope
IAC are applied to namespaces, wikipages, and media files, collectively called scopes. IAC may be applied to pages as a more specific option that overrides its parent namespace permissions. For example, a `/development/` namespace may have a PV of `ALL;R`. But a page under this namespace, say `/development/onboarding` may have a PV of `ALL;U`. Regardless of the parent namespace's PV, all users will receive the PV of U for this page. However, sibling pages will inherit the `ALL;R` permission set by the parent namespace unless otherwise configured.

Additionally, if `/development/` has a PV of `contributors:N` (i.e. users in the group `contributors` cannot even read any files under the `/development` namespace), and a wikipage `/development/onboarding` has a PV of `ALL;R`, then users in the group `contributors` are able to read this page, even though the namespace forbids this. Why? This is because of the specificity rule in IAC - the most specific rule overrides all other rules. Check §The Algorithm section for more information on the specificity rule.

As the name of the protocol (Inherited Access Controls) suggests, all child pages under a namespace, no matter how deep, will inherit their parent's PV when they have no matching PV of their own.

## Usergroup
Usergroups are groups of users, and users are authenticated and unauthenticated individuals. IAC has default usergroups named `ALL`, `ALL_UNAUTH`, and `ALL_AUTH`. These usergroups are automatically assigned to all users. `ALL` contains, fittingly, all users, `ALL_AUTH` includes all logged-in users, and `ALL_UNAUTH` includes all logged-out users. `ALL_UNAUTH` is simply a fallback for when `ALL_AUTH` fails to catch remaining, unauthorised end users.

## The Algorithm
When any scope is requested, IAC determines the requester's EPV by walking outward from the requested scope toward the root namespace, stopping at the first scope where a matching grant is found.

The walk proceeds as follows:
1. Start at the requested scope.
2. Gather all grants at the current scope that match the requester. A grant matches if it names the requester directly `user:alice` or names a usergroup the requester belongs to (including default usergroups: `ALL`, `ALL_AUTH`, `ALL_UNAUTH`). There is no priority between user-grants and usergroup-grants — both are simply candidate matches at this scope.
3. If one or more grants matched at the current scope:
  - If exactly one grant matched, its PV becomes the requester's EPV. Resolution stops here.
  - If multiple grants matched (e.g. the requester belongs to several usergroups with different PVs, or matches both a user-grant and a usergroup-grant), the highest PV among them becomes the EPV, per the PV hierarchy (D > C > U > R > N). Resolution stops here.
  - Once a matching scope is found, no further (farther) scopes are consulted, regardless of what PV they would have granted.
  4. If no grants matched at the current scope, move to the next-closest ancestor namespace (the immediate parent of the current scope) and return to step 2.
  5. If no grant matches at any scope, including root, the EPV defaults to N. This should only occur if the wiki is misconfigured (root should always have at least an ALL grant); administrators should treat an unmatched root as a configuration error to fix.
  6. The resulting EPV governs the requester's access to the originally requested scope — the page/namespace/file where the walk began, not the scope where the match was ultimately found.

A closer scope with any matching grant always wins over a farther scope, even if the farther scope's grant would have been more permissive. See Example: Page-level grant re-opens a namespace-level blockage.




### Specificity examples

#### Basic scope resolution
`/development` grants EPV: `ALL;R`. `/development/onboarding` has no grants at all.
No grant matches at the page level → inherit `/development` EPV -> `/development/onboarding` grants EPV: `ALL;R`.

#### Highest-grant-wins among multiple matches at the same scope
`/development` grants `foo:R` and `bar:U`. `user:Bob` is in both `usergroup:foo` and `usergroup:bar`.
Both grants match at the same scope → take the highest → EPV: `bar:U` (and, per the PV ladder, R too, since U implies R).

#### Page-level grant re-opens a namespace-level blockage
`/development` grants `Contributors;N`. `/development/onboarding` grants `ALL;R`. `user:Alice` is in `usergroups:Contributors`.
Page-level scope has a match (`user:Alice` is a member of `usergroup:ALL`) → EPV: `R`. The namespace-level `Contributors;N` never gets consulted because a closer-scope match already existed.

#### User EPV at higher namespace contradicting lower namespace EPV
`/development` grants `alice;R`. `/development/github` grants `Contributors;C`. Alice is in Contributors.
`At /development/github`, Alice is matched into Contributors;C, since she's a member. → Scope-level match found at` /development/github` → don't consult `/development/alice:R` at all → EPV: C. Scope proximity alone decided this. Even though the individual was granted a different EPV at a higher namespace, since IAC is a Specificity-Precedence Protocol (SPP) (cf. Abstraction-Precedence Protocol, or APP, where higher level namespaces are consulted first), they receive C as their EPV since a page-level EPV grant is more _specific_.

## Base user permission
Upon Mast installation, the administrator sets the `base_user_permission` variable in the config. This variable defines which PV the `ALL` usergroup receives for each namespace. Changing this value post-installation does not retroactively affect namespaces created in the past with a different `base_user_permission` variable, only ones created after the change.

Additionally, depending on this variable, the `ALL` usergroup is assigned to the root namespace. Changing the variable will retroactively change the root namespace permission.

## Bypass
The creator of the wiki instance (i.e. the one who installed the wiki on the server) receives a special PV: `S`. This is the `sudo` permission. This allows the user to perform any action on any namespace without any restrictions. Only a user who posseses the `sudo` permission can assign this role to other users.
