rule SecretsLeakage
{
    meta:
        name = "Secrets Leakage Detection"
        description = "Detects potential exposure of sensitive information like API keys, passwords, and tokens"
        severity = "HIGH"
        category = "secrets,security,data-leakage,credentials"
        author = "Ramparts Security Team"
        version = "1.0"
        
    strings:
        $api_key = /[Aa][Pp][Ii][-_]?[Kk][Ee][Yy].*[A-Za-z0-9]{20,}/
        $bearer_token = /[Bb]earer\s+[A-Za-z0-9\-_]{20,}/
        $password = /[Pp]assword.*[A-Za-z0-9@#$%^&*]{8,}/
        $private_key = /-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----/
        $aws_key = /AKIA[0-9A-Z]{16}/
        $github_token = /ghp_[A-Za-z0-9]{36}/
    condition:
        $api_key or $bearer_token or $password or $private_key or $aws_key or $github_token
}

rule SSHKeyExposure
{
    meta:
        name = "SSH Key Exposure"
        description = "Detects SSH keys, authorized_keys files, and SSH configuration access"
        severity = "CRITICAL" 
        category = "ssh,security,credentials,access"
        author = "Ramparts Security Team"
        version = "1.0"
        
    strings:
        // SSH private key patterns
        $ssh_rsa_key = "-----BEGIN RSA PRIVATE KEY-----"
        $ssh_ed25519_key = "-----BEGIN OPENSSH PRIVATE KEY-----"
        $ssh_ecdsa_key = "-----BEGIN EC PRIVATE KEY-----"
        $ssh_dsa_key = "-----BEGIN DSA PRIVATE KEY-----"
        
        // SSH file paths and configurations
        $ssh_dir = /.ssh[\/\\]/
        $authorized_keys = "authorized_keys"
        $id_rsa = "id_rsa"
        $id_ed25519 = "id_ed25519"
        $id_ecdsa = "id_ecdsa"
        $id_dsa = "id_dsa"
        $known_hosts = "known_hosts"
        $ssh_config = /ssh[_-]?config/i
        
        // SSH public key formats
        $ssh_rsa_pub = /ssh-rsa\s+[A-Za-z0-9+\/=]+/
        $ssh_ed25519_pub = /ssh-ed25519\s+[A-Za-z0-9+\/=]+/
        $ssh_ecdsa_pub = /ecdsa-sha2-[0-9]+\s+[A-Za-z0-9+\/=]+/
        
    condition:
        any of ($ssh_rsa_key, $ssh_ed25519_key, $ssh_ecdsa_key, $ssh_dsa_key) or
        any of ($ssh_dir, $authorized_keys, $id_rsa, $id_ed25519, $id_ecdsa, $id_dsa, $known_hosts, $ssh_config) or
        any of ($ssh_rsa_pub, $ssh_ed25519_pub, $ssh_ecdsa_pub)
}

rule PEMFileAccess
{
    meta:
        name = "PEM File Access"
        description = "Detects access to PEM certificate files and private keys"
        severity = "CRITICAL"
        category = "certificates,security,pem,crypto"
        author = "Ramparts Security Team"
        version = "1.0"
        
    strings:
        // PEM certificate headers
        $pem_cert = "-----BEGIN CERTIFICATE-----"
        $pem_private_key = "-----BEGIN PRIVATE KEY-----"
        $pem_rsa_private = "-----BEGIN RSA PRIVATE KEY-----"
        $pem_encrypted_private = "-----BEGIN ENCRYPTED PRIVATE KEY-----"
        $pem_ec_private = "-----BEGIN EC PRIVATE KEY-----"
        $pem_dsa_private = "-----BEGIN DSA PRIVATE KEY-----"
        $pem_public_key = "-----BEGIN PUBLIC KEY-----"
        $pem_rsa_public = "-----BEGIN RSA PUBLIC KEY-----"
        
        // Certificate file extensions
        $pem_ext = /\.(pem|crt|cer|key|p12|pfx|jks)(\"|\'|\s|$)/i
        
        // SSL/TLS related patterns
        $ssl_cert = /ssl[_-]?cert/i
        $tls_cert = /tls[_-]?cert/i
        $ca_cert = /ca[_-]?cert/i
        $server_cert = /server[_-]?cert/i
        $client_cert = /client[_-]?cert/i
        
    condition:
        any of ($pem_cert, $pem_private_key, $pem_rsa_private, $pem_encrypted_private, $pem_ec_private, $pem_dsa_private, $pem_public_key, $pem_rsa_public) or
        $pem_ext or
        any of ($ssl_cert, $tls_cert, $ca_cert, $server_cert, $client_cert)
}

rule EnvironmentVariableLeakage
{
    meta:
        name = "Environment Variable Leakage"
        description = "Detects exposure of sensitive environment variables and API keys"
        severity = "HIGH"
        category = "environment,secrets,api-keys,credentials"
        author = "Ramparts Security Team"
        version = "1.0"
        
    strings:
        // Generic sensitive environment variables
        $env_api_key = /[A-Z_]*API[_-]?KEY[A-Z_]*/i
        $env_secret = /[A-Z_]*SECRET[A-Z_]*/i
        $env_password = /[A-Z_]*PASSWORD[A-Z_]*/i
        $env_token = /[A-Z_]*TOKEN[A-Z_]*/i
        $env_auth = /[A-Z_]*AUTH[A-Z_]*/i
        
        // Specific service API keys
        $aws_access_key = /AWS_ACCESS_KEY_ID/i
        $aws_secret_key = /AWS_SECRET_ACCESS_KEY/i
        $github_token = /GITHUB_TOKEN/i
        $openai_key = /OPENAI_API_KEY/i
        $anthropic_key = /ANTHROPIC_API_KEY/i
        $google_api_key = /GOOGLE_API_KEY/i
        $stripe_key = /STRIPE_[A-Z_]*KEY/i
        
        // Database credentials
        $db_password = /DB_PASSWORD/i
        $database_url = /DATABASE_URL/i
        $redis_url = /REDIS_URL/i
        
        // Environment variable patterns with values
        $env_with_value = /[A-Z_]+\s*=\s*["\']?[A-Za-z0-9+\/=@#$%^&*\-_.]{10,}["\']?/
        
        // Process environment access
        $process_env = /process\.env\./
        $os_environ = /os\.environ/
        $getenv = /getenv\s*\(/
        $env_var = /\$\{?[A-Z_]+\}?/
        
    condition:
        any of ($env_api_key, $env_secret, $env_password, $env_token, $env_auth) or
        any of ($aws_access_key, $aws_secret_key, $github_token, $openai_key, $anthropic_key, $google_api_key, $stripe_key) or
        any of ($db_password, $database_url, $redis_url) or
        $env_with_value or
        any of ($process_env, $os_environ, $getenv, $env_var)
}