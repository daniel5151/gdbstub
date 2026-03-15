#!/bin/bash

set -e

# Script to enforce that each Target method has a corresponding entry in impl_dyn_target

# Get the list of methods in the Target trait
target_methods=$(awk '/pub trait Target {/,/^}/' src/target/mod.rs | grep '^    fn ' | sed 's/    fn \([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/')

# Get the list of delegated methods in impl_dyn_target
delegated_methods=$(awk '/macro_rules! impl_dyn_target {/,/^}/' src/target/mod.rs | grep '__delegate!\|__delegate_support!' | sed -n 's/.*fn \([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/p; s/__delegate_support!(\([a-zA-Z_][a-zA-Z0-9_]*\).*/support_\1/p' | sed 's/^ *//')

# Check for missing delegations
missing=""
for method in $target_methods; do
    if ! echo "$delegated_methods" | grep -q "^$method$"; then
        missing="$missing $method"
    fi
done

if [ -n "$missing" ]; then
    echo "Error: Missing delegations for methods:$missing"
    exit 1
fi

echo "All Target methods have corresponding delegations in impl_dyn_target."
