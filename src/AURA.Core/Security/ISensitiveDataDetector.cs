namespace Aura.Core.Security;

public interface ISensitiveDataDetector
{
    IReadOnlyList<SensitiveDataMatch> Detect(string text);
}
