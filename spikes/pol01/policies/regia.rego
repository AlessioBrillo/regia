# POL-01 policy set (Rego). Mirrors src/oracle.rs. Undefined is false, so a missing attribute never allows;
# the adapter additionally maps any evaluation error to a deny.
package regia

import rego.v1

default allow := false

allow if {
	count(deny) == 0
	count(permit) > 0
}

default reasons := {"deny_default"}

reasons := deny if count(deny) > 0

reasons := permit if {
	count(deny) == 0
	count(permit) > 0
}

mutating := {"create", "update", "delete", "export", "share", "execute"}

view := {"read", "discover"}

known_label := {"public", "circle", "restricted", "secret"}

excluded := {"health", "minors", "finance", "credentials", "id_document"}

own_role := r if {
	some m in input.principal.memberships
	m.circle == input.resource.circle
	not m.revoked
	r := m.role
}

is_owner if input.resource.owner == input.principal.id

is_grantee if input.principal.id in input.resource.grantees

circle_like if input.resource.label in {"circle", "public"}

deny contains "deny_missing_or_unknown_label" if not input.resource.label in known_label

deny contains "deny_revoked" if {
	some m in input.principal.memberships
	m.circle == input.resource.circle
	m.revoked
}

deny contains "deny_excluded_category" if {
	input.action == "create"
	input.resource.category in excluded
}

deny contains "deny_unapproved_untrusted_mutation" if {
	input.action in mutating
	input.context.untrusted
	not input.context.approved
}

permit contains "allow_secret_owner" if {
	input.resource.label == "secret"
	is_owner
}

permit contains "allow_restricted_owner" if {
	input.resource.label == "restricted"
	is_owner
}

permit contains "allow_restricted_grantee" if {
	input.resource.label == "restricted"
	input.action in view
	is_grantee
}

permit contains "allow_public_view" if {
	input.resource.label == "public"
	input.action in view
}

permit contains "allow_circle_owner" if {
	circle_like
	own_role == "owner"
}

permit contains "allow_circle_member" if {
	circle_like
	own_role == "member"
	input.action in {"read", "discover", "create", "update", "execute"}
}

permit contains "allow_named_guest" if {
	circle_like
	own_role == "guest"
	input.action in view
	is_grantee
}

permit contains "allow_ancestor_owner_view" if {
	circle_like
	input.action in view
	some a in input.resource.ancestors
	some m in input.principal.memberships
	m.circle == a
	not m.revoked
	m.role == "owner"
}
