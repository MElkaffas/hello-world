namespace Aura.Host.Configuration;

/// <summary>Expands the small set of path tokens used in <see cref="AuraConfiguration"/> so
/// config files stay portable across machines/OSes rather than embedding an absolute path.</summary>
public static class AuraPaths
{
    public static string Expand(string path)
    {
        var localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        return path.Replace("{localAppData}", localAppData, StringComparison.Ordinal);
    }

    /// <summary>Resolves a possibly-relative directory against the app's base directory, so the
    /// same config works whether rules ship alongside the exe or at an absolute tenant path.</summary>
    public static string ResolveDirectory(string path)
    {
        var expanded = Expand(path);
        return Path.IsPathRooted(expanded) ? expanded : Path.Combine(AppContext.BaseDirectory, expanded);
    }
}
