#!/bin/bash

# Generate SSH host keys if they do not exist
if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A
fi

# Run the SSH daemon in the foreground
echo "Starting SSH server..."
exec /usr/sbin/sshd -D
