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