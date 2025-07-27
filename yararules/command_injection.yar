/*
 * Command Injection Detection Rule
 * 
 * This rule detects various command injection attack patterns including:
 * - Shell command separators and operators
 * - Dangerous system commands
 * - Code execution functions
 * - Evasion techniques (encoding, obfuscation)
 * - Command chaining patterns
 * - File system manipulation commands
 * - Network and process control commands
 * - Privilege escalation attempts
 * 
 * Updated to be more specific and avoid false positives on legitimate tool names
 */

rule CommandInjection
{
    meta:
        name = "Advanced Command Injection Detection"
        author = "Ramparts Security Team"
        date = "2024-07-25"
        version = "2.1"
        description = "Comprehensive command injection detection covering multiple attack vectors and evasion techniques"
        severity = "CRITICAL"
        category = "command-injection,security,code-execution,privilege-escalation,data-exfiltration,reverse-shell,web-shell,evasion-detection"
        confidence = "HIGH"
        
    strings:
        // Shell command separators and operators (more specific patterns)
        $shell_separators = /[;&|`$(){}]/
        $command_chaining = /(&&|\|\||;|`|\$\(|\)|{|})/
        $pipe_operators = /\|/
        $background_exec = /&/
        
        // Dangerous system commands (file operations) - more specific
        $file_dangerous = /(rm\s+-rf?|del\s+/i|format\s+|dd\s+if|mkfs|fdisk|wipefs)/
        $file_manipulation = /(chmod\s+777|chown\s+root|chgrp\s+root|touch\s+.*\.sh|echo\s+.*\>.*\.sh)/
        
        // Dangerous system commands (process control) - more specific
        $process_dangerous = /(kill\s+-9|killall|pkill|kill\s+1|shutdown\s+-h|reboot|halt)/
        $process_control = /(ps\s+aux|top|htop|pstree|pgrep|pidof)/
        
        // Dangerous system commands (network) - more specific
        $network_dangerous = /(nc\s+-l|netcat|telnet|ssh\s+-o|wget\s+.*\||curl\s+.*\||ftp\s+get)/
        $network_tools = /(nmap|nslookup|dig|ping\s+-c|traceroute|route\s+add)/
        
        // Code execution functions (multiple languages) - more specific patterns
        $exec_functions = /\b(system|exec|popen|spawn|eval|shell_exec|passthru|proc_open)\b/
        $python_exec = /\b(os\.system|subprocess\.|exec\(|eval\(|compile\(|execfile\(|input\(\))\b/
        $node_exec = /\b(child_process\.|exec\(|spawn\(|execSync\(|spawnSync\(|require\(|eval\(\))\b/
        $php_exec = /\b(shell_exec|exec|system|passthru|proc_open|popen|eval|assert|create_function)\b/
        $java_exec = /\b(Runtime\.getRuntime\(\)\.exec|ProcessBuilder|ScriptEngine|eval\(|exec\(\))\b/
        
        // Evasion techniques - more specific
        $encoding_evasion = /(base64\s+-d|base64\s+decode|echo\s+.*\||printf\s+.*\||xxd\s+-r)/
        $obfuscation = /(eval\s+\$|eval\s+`|eval\s+\$\(|eval\s+base64|eval\s+printf)/
        $variable_substitution = /\$\{[^}]+\}|\$\([^)]+\)/
        
        // Command injection patterns - more specific
        $injection_patterns = /(\$\{[^}]+\}|\$\([^)]+\)|`[^`]+`|eval\s+.*\$|exec\s+.*\$|system\s+.*\$)/
        
        // Privilege escalation - more specific
        $privilege_escalation = /(sudo\s+.*|su\s+.*|chmod\s+4755|chmod\s+6755|setuid|setgid)/
        
        // Data exfiltration - more specific
        $data_exfil = /(cat\s+.*\.(passwd|shadow|config|env|key|pem|p12|pfx)|grep\s+.*password|find\s+.*-name\s+.*\.(key|pem|p12))/
        
        // Reverse shell patterns - more specific
        $reverse_shell = /(bash\s+-i\s*>\s*&|nc\s+-e|telnet\s+.*\||/bin/bash\s+-i|/bin/sh\s+-i)/
        
        // Web shell indicators - more specific
        $web_shell = /(php\s+-r|python\s+-c|perl\s+-e|ruby\s+-e|node\s+-e)/
        
        // Suspicious command combinations - more specific
        $suspicious_combo = /(rm\s+.*&&|del\s+.*&&|format\s+.*&&|kill\s+.*&&|shutdown\s+.*&&)/
        
        // Legitimate patterns to exclude (avoid false positives)
        $legitimate_patterns = /(create_file|update_file|read_file|write_file|push_files|git_|file_|add_comment|list_commits)/
        
    condition:
        // Primary detection: shell separators with dangerous commands
        ($shell_separators and ($file_dangerous or $process_dangerous or $network_dangerous)) or
        
        // Code execution functions (but exclude legitimate patterns)
        ($exec_functions and not $legitimate_patterns) or
        
        // Language-specific execution (but exclude legitimate patterns)
        (($python_exec or $node_exec or $php_exec or $java_exec) and not $legitimate_patterns) or
        
        // Evasion techniques
        ($encoding_evasion and $shell_separators) or
        ($obfuscation and $shell_separators) or
        
        // Command injection patterns
        $injection_patterns or
        
        // Privilege escalation
        $privilege_escalation or
        
        // Data exfiltration
        $data_exfil or
        
        // Reverse shell
        $reverse_shell or
        
        // Web shell
        $web_shell or
        
        // Suspicious combinations
        $suspicious_combo
} 