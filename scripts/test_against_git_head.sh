#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <git-commit-id>"
  exit 1
fi

COMMIT="$1"

COMMANDS=(
  'target/release/openschafkopf suggest-card --rules "Rufspiel mit der Eichel-Sau von 1" --cards-on-table "EK EZ HA EA  SZ S9 SK" --hand "SU HK E9 SA  EO HO SO GU G9  HU H9 GA GZ  GO EU HZ GK" --hand "EO HO SO GU G9" --branching oracle --snapshotcache --constrain-hands "ctx.who_has_sa()!=3 && ctx.trumpf(2)>=2 && ctx.trumpf(3)==3 && ctx.who_has_su()==0"'
  'target/release/openschafkopf suggest-card --rules "Rufspiel mit der Gras-Sau von 1" --cards-on-table "EK EZ HA EA  SZ S9 SK" --hand "SU HK E9 SA  EO HO SO GU G9  HU H9 GA GZ  GO EU HZ GK" --hand "EO HO SO GU G9" --branching oracle --snapshotcache --constrain-hands "ctx.who_has_sa()!=3 && ctx.trumpf(2)>=2 && ctx.trumpf(3)==3 && ctx.who_has_su()==0"'
  'target/release/openschafkopf suggest-card --rules "Herz-Solo von 2" --cards-on-table "EK EZ HA EA  SZ S9 SK" --hand "SU HK E9 SA  EO HO SO GU G9  HU H9 GA GZ  GO EU HZ GK" --hand "EO HO SO GU G9" --branching oracle --snapshotcache --constrain-hands "ctx.who_has_sa()!=3 && ctx.trumpf(2)>=2 && ctx.trumpf(3)==3 && ctx.who_has_su()==0"'
)

TMPDIR=$(mktemp -d)
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

cleanup() {
  git checkout -q "$CURRENT_BRANCH"
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

build() {
  cargo build -j8 --release 2>&1
}

run_commands() {
  local prefix="$1"
  local i=0
  for cmd in "${COMMANDS[@]}"; do
    eval "$cmd" >"$TMPDIR/${prefix}_${i}.out" 2>&1
    ((++i))
  done
}

echo "Building HEAD..."
build
echo "Running commands on HEAD..."
run_commands head

echo "Checking out $COMMIT..."
git checkout -q "$COMMIT"

echo "Building $COMMIT..."
build
echo "Running commands on $COMMIT..."
run_commands old

echo "Comparing outputs..."
for i in "${!COMMANDS[@]}"; do
  cmd="${COMMANDS[$i]}"
  if diff -u "$TMPDIR/old_${i}.out" "$TMPDIR/head_${i}.out"; then
    echo "OK [$i]: $cmd"
  else
    echo "FAIL [$i]: $cmd"
    exit 1
  fi
done

echo "All outputs match."
