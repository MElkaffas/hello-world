using Aura.Core.Actions;
using Aura.Core.Connectors;

namespace Aura.Host.Connectors;

/// <summary>
/// The two factories the default configuration enables. Real connector factories
/// (Graph-backed Outlook, OpenXML-backed Word) are a natural next registration once their
/// per-tenant settings (app registration, document profile source) are designed - deliberately
/// not added here yet, so this phase stays scoped to proving the registry mechanism itself.
/// </summary>
public sealed class MockOutlookConnectorFactory : IConnectorFactory
{
    public string Implementation => "mock-outlook";

    public IConnector CreateConnector(IReadOnlyDictionary<string, string> settings) =>
        MockConnector.OutlookComposeMissingSubjectAndAttachment();

    public IActionHandler? CreateActionHandler(IReadOnlyDictionary<string, string> settings) =>
        new MockActionHandler("outlook");
}

public sealed class MockWordConnectorFactory : IConnectorFactory
{
    public string Implementation => "mock-word";

    public IConnector CreateConnector(IReadOnlyDictionary<string, string> settings) =>
        MockConnector.WordDocumentOffProfile();

    public IActionHandler? CreateActionHandler(IReadOnlyDictionary<string, string> settings) =>
        new MockActionHandler("word");
}
