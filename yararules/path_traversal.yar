rule PathTraversalVulnerability
{
    meta:
        name = "Path Traversal Vulnerability"
        description = "Detects potential path traversal patterns and unsafe file access"
        severity = "HIGH"
        category = "path-traversal,security,file-access"
        author = "Ramparts Security Team"
        version = "1.0"
        
    strings:
        // Directory traversal patterns
        $dot_dot_slash = "../"
        $dot_dot_backslash = "..\\"
        $encoded_dot_dot = /%2e%2e%2f/i
        $double_encoded = /%252e%252e%252f/i
        
        // Sensitive system paths
        $etc_path = "/etc/"
        $root_path = "/root/"
        $var_path = "/var/"
        $proc_path = "/proc/"
        $sys_path = "/sys/"
        $windows_system = /[Cc]:[\\\/][Ww]indows[\\\/]/
        $windows_users = /[Cc]:[\\\/][Uu]sers[\\\/]/
        
        // Unsafe file operations
        $file_read = /file_get_contents|readFile|fopen|open\s*\(/
        $file_include = /include|require|import/
        
    condition:
        any of ($dot_dot_slash, $dot_dot_backslash, $encoded_dot_dot, $double_encoded) or
        any of ($etc_path, $root_path, $var_path, $proc_path, $sys_path, $windows_system, $windows_users) or
        ($file_read and any of ($dot_dot_slash, $dot_dot_backslash)) or
        ($file_include and any of ($dot_dot_slash, $dot_dot_backslash))
} 