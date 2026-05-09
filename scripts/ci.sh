#!/bin/bash

set -e

cd "$(dirname "$0")/.." || exit 1

RUN_TEST=true
RUN_RUSTFMT=true

if [ $# -gt 0 ]; then
    RUN_TEST=false
    RUN_RUSTFMT=false
    for arg in "$@"; do
        case "$arg" in
            test) RUN_TEST=true ;;
            rustfmt) RUN_RUSTFMT=true ;;
            *)
                echo -e "\033[0;31mError: Unknown argument: $arg\033[0m"
                echo "Usage: $0 [test] [rustfmt]"
                exit 1
                ;;
        esac
    done
fi

if [ "${GITHUB_ACTIONS:-false}" = "true" ] || [ "${CI:-false}" = "true" ]; then
    IS_CI=true
else
    IS_CI=false
fi

if [ -t 1 ] || [ "$IS_CI" = true ]; then
    RED='\033[0;31m'
    YELLOW='\033[1;33m'
    GREEN='\033[0;32m'
    NC='\033[0m' # No Color
else
    RED=''
    YELLOW=''
    GREEN=''
    NC=''
fi

# Logging helpers
log_info() {
    echo -e "$*"
}

log_warning() {
    if [ "$IS_CI" = true ] && [ -n "$GITHUB_ACTIONS" ]; then
        echo "::warning::$*"
    else
        echo -e "${YELLOW}$*${NC}"
    fi
}

log_error() {
    if [ "$IS_CI" = true ] && [ -n "$GITHUB_ACTIONS" ]; then
        echo "::error::$*"
    else
        echo -e "${RED}$*${NC}"
    fi
}

log_success() {
    echo -e "${GREEN}$*${NC}"
}

# Grouping helpers
start_group() {
    local name="$1"
    if [ "$IS_CI" = true ] && [ -n "$GITHUB_ACTIONS" ]; then
        echo "::group::$name"
    else
        echo -e "\n${GREEN}=== $name ===${NC}"
    fi
}

end_group() {
    if [ "$IS_CI" = true ] && [ -n "$GITHUB_ACTIONS" ]; then
        echo "::endgroup::"
    fi
}

# Error handling functions
check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "Error: '$1' is not installed"
        log_info "\nInstallation instructions:"
        case "$1" in
            cargo|rustc)
                log_info "  Install Rust by visiting: https://rustup.rs/"
                log_info "  Or run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
                ;;
            bash)
                log_info "  Install bash: see your system package manager"
                ;;
        esac
        log_info ""
        exit 1
    fi
}

check_toolchain() {
    local toolchain="$1"
    if ! rustup toolchain list | grep -q "^$toolchain"; then
        log_error "Error: Rust toolchain '$toolchain' is not installed"
        log_info "\nTo install it, run:"
        log_warning "  rustup toolchain install $toolchain --profile minimal"
        if [ "$toolchain" = "nightly" ]; then
            log_warning "  rustup component add rustfmt --toolchain=$toolchain"
        fi
        log_info ""
        exit 1
    fi
}

if [ "$IS_CI" = false ]; then
    log_warning "Running CI checks locally\n"
    check_command "cargo"
    check_command "rustc"

    if [ "$RUN_TEST" = true ]; then
        check_toolchain "stable"
    fi
    if [ "$RUN_RUSTFMT" = true ]; then
        check_toolchain "nightly"
    fi
fi

run_step() {
    local step_name="$1"
    shift

    start_group "$step_name"

    # Evaluate the command
    if "$@"; then
        log_success "[✓] $step_name"
        # Always end the group on success
        end_group
        log_info ""
    else
        log_error "$step_name failed"
        end_group
        exit 1
    fi
}

log_info "=========================================="
log_info "  gdbstub CI"
log_info "==========================================\n"

# ============================================
# RUSTFMT JOB
# ============================================
if [ "$RUN_RUSTFMT" = true ]; then
    start_group "RUSTFMT JOB - nightly format check"
    log_info ""

    run_step "cargo fmt check (all)" \
        cargo +nightly fmt --all -- --check

    run_step "cargo fmt check (example_no_std)" \
        cargo +nightly fmt --manifest-path example_no_std/Cargo.toml -- --check

    end_group
    log_info ""
fi

# ============================================
# TEST JOB
# ============================================
if [ "$RUN_TEST" = true ]; then
    start_group "TEST JOB - clippy + tests + docs"
    log_info ""

    run_step "cargo clippy (workspace)" \
        cargo +stable clippy --workspace --tests --examples --features=std -- -D warnings

    run_step "cargo clippy (example_no_std)" \
        cargo +stable clippy --manifest-path example_no_std/Cargo.toml

    # Because we `cd` to the root earlier, these relative paths are now completely safe
    run_step "check dyn Target delegations" \
        bash ./scripts/check_target_delegation.sh

    run_step "cargo test" \
        cargo +stable test --workspace --features=std

    run_step "no panics in example_no_std" \
        bash ./example_no_std/dump_asm.sh

    run_step "cargo doc" \
        bash -c 'RUSTDOCFLAGS="-Dwarnings" cargo +stable doc --workspace --features=std'

    end_group
    log_info ""
fi

log_success "=========================================="
log_success "  All checks passed!"
log_success "=========================================="
