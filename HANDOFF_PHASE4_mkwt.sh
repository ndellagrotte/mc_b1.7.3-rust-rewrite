#!/bin/sh
# usage: mkwt.sh <wp-id> <base-ref>   -> .worktrees/wp-<id> on branch wp-<id>, .reference symlink, reflinked target/
set -e
ROOT=/home/nick/IdeaProjects/mc_b1.7.3-rust-rewrite
cd "$ROOT"
git worktree add -q -b "wp-$1" ".worktrees/wp-$1" "$2"
ln -s "$ROOT/.reference" ".worktrees/wp-$1/.reference"
cp -r --reflink=always target ".worktrees/wp-$1/target"
echo "ok wp-$1 at $(git -C .worktrees/wp-$1 rev-parse --short HEAD)"
