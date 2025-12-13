#!/bin/sh
# Provision directories, PAM entry, and SMF manifest for yanOS.
set -e

# Directories and config
mkdir -p /etc/opt/yanos /etc/opt/yanos/tls /opt/yanos/ui /var/opt/yanos
chmod 700 /etc/opt/yanos /etc/opt/yanos/tls

# Config file placeholder
if [ ! -f /etc/opt/yanos/config.json ]; then
  umask 077
  echo '{}' > /etc/opt/yanos/config.json
fi
chmod 600 /etc/opt/yanos/config.json

# Session key
if [ ! -f /etc/opt/yanos/session.key ]; then
  umask 077
  head -c 64 /dev/urandom > /etc/opt/yanos/session.key
fi
chmod 600 /etc/opt/yanos/session.key

# TLS placeholders (binary will self-generate if missing)
touch /etc/opt/yanos/tls/cert.pem /etc/opt/yanos/tls/key.pem
chmod 600 /etc/opt/yanos/tls/cert.pem /etc/opt/yanos/tls/key.pem

# PAM stack entry for service "yanos"
if ! grep -q "^yanos[[:space:]]" /etc/pam.conf; then
  cat <<'EOF' >> /etc/pam.conf
yanos   auth        requisite   pam_authtok_get.so.1
yanos   auth        required    pam_unix_auth.so.1
yanos   account     requisite   pam_roles.so.1
yanos   account     required    pam_unix_account.so.1
yanos   session     required    pam_unix_session.so.1
EOF
fi

# Service user (optional; adjust UID/GID as needed)
if ! getent passwd yanos >/dev/null 2>&1; then
  useradd -g noaccess -d /var/opt/yanos -s /usr/bin/false yanos || true
fi

# Import SMF manifest if installed
if [ -f /opt/yanos/manifest/yanos.xml ]; then
  mkdir -p /var/svc/manifest/site
  cp /opt/yanos/manifest/yanos.xml /var/svc/manifest/site/
  svccfg import /var/svc/manifest/site/yanos.xml
fi
