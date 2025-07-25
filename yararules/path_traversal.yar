rule PathTraversal
{
    meta:
        name = "Path Traversal Detection"
        description = "Detects path traversal patterns that could lead to unauthorized file access"
        severity = "MEDIUM"
        category = "path-traversal,security,file-access"
        author = "Ramparts Security Team"
        version = "1.0"
        
    strings:
        $dotdot = "../"
        $dotdot_win = "..\\"
        $absolute_path = "/etc/"
        $windows_path = "C:\\"
    condition:
        $dotdot or $dotdot_win or $absolute_path or $windows_path
} 