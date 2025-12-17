#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 user <username> [keytab_path]" >&2
  echo "       $0 host <hostname> [keytab_path]" >&2
  echo "Examples:" >&2
  echo "  $0 user alice /tmp/alice.keytab" >&2
  echo "  $0 host demo-server.UD.INTERNAL /etc/krb5.keytab" >&2
  exit 1
}

if [[ $# -lt 2 ]]; then
  usage
fi

type="$1"
principal="$2"
keytab="${3:-/etc/krb5.keytab}"

case "$type" in
  user)
    echo "# Run inside the KDC host/container (kadmin.local)"
    echo "kadmin.local -q \"addprinc -randkey ${principal}@UD.INTERNAL\""
    echo "kadmin.local -q \"ktadd -k ${keytab} ${principal}@UD.INTERNAL\""
    ;;
  host)
    echo "# Run inside the KDC host/container (kadmin.local)"
    echo "kadmin.local -q \"addprinc -randkey host/${principal}@UD.INTERNAL\""
    echo "kadmin.local -q \"ktadd -k ${keytab} host/${principal}@UD.INTERNAL\""
    ;;
  *)
    usage
    ;;
esac

echo "# Securely copy the keytab to the target and set owner/permissions"
echo "chmod 600 ${keytab}"
