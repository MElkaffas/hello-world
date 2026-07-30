namespace Aura.Core.Security;

public interface IRedactor
{
    string Redact(string text, IReadOnlyList<SensitiveDataMatch> matches);
}
