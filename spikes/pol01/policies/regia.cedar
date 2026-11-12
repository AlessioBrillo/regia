// POL-01 policy set (Cedar). Mirrors the rules documented in src/oracle.rs.
// Every optional attribute is guarded with `has`: Cedar skips a policy that errors, and a skipped forbid would
// fail open, so the adapter also treats any evaluation error as a deny.

@id("deny_revoked")
forbid (principal, action, resource) when { principal in resource.revoked };

@id("deny_excluded_category")
forbid (principal, action == Action::"create", resource) when {
  resource has category &&
  ["health", "minors", "finance", "credentials", "id_document"].contains(resource.category)
};

@id("deny_unapproved_untrusted_mutation")
forbid (principal, action in Action::"mutate", resource) when { context.untrusted && !context.approved };

@id("allow_secret_owner")
permit (principal, action, resource) when {
  resource has label && resource.label == "secret" && resource has owner && principal == resource.owner
};

@id("allow_restricted_owner")
permit (principal, action, resource) when {
  resource has label && resource.label == "restricted" && resource has owner && principal == resource.owner
};

@id("allow_restricted_grantee")
permit (principal, action in Action::"view", resource) when {
  resource has label && resource.label == "restricted" && resource.grantees.contains(principal)
};

@id("allow_public_view")
permit (principal, action in Action::"view", resource) when {
  resource has label && resource.label == "public"
};

@id("allow_circle_owner")
permit (principal, action, resource) when {
  resource has label && ["circle", "public"].contains(resource.label) && principal in resource.owners
};

@id("allow_circle_member")
permit (principal, action in [Action::"read", Action::"discover", Action::"create", Action::"update", Action::"execute"], resource) when {
  resource has label && ["circle", "public"].contains(resource.label) && principal in resource.members
};

@id("allow_named_guest")
permit (principal, action in Action::"view", resource) when {
  resource has label && ["circle", "public"].contains(resource.label) &&
  principal in resource.guests && resource.grantees.contains(principal)
};

@id("allow_ancestor_owner_view")
permit (principal, action in Action::"view", resource) when {
  resource has label && ["circle", "public"].contains(resource.label) && principal in resource.ancestor_owners
};
