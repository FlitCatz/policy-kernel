using System.Net;
using System.Text.Json;
using Microsoft.AspNetCore.Mvc.Testing;
using Xunit;

namespace Consumer.Tests;
public sealed class ConsumerTests
{
    [Fact, Trait("Suite", "Unit")]
    public void Greeting_is_stable() => Assert.Equal("Hello policy", Greeting.Message);

    [Fact, Trait("Suite", "Integration")]
    public async Task Endpoint_returns_success()
    {
        await using var app = new WebApplicationFactory<Program>();
        using var client = app.CreateClient();
        using var response = await client.GetAsync("/api/greeting");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }

    [Fact, Trait("Suite", "Contract")]
    public async Task Endpoint_preserves_wire_contract()
    {
        await using var app = new WebApplicationFactory<Program>();
        using var client = app.CreateClient();
        using var document = JsonDocument.Parse(await client.GetStringAsync("/api/greeting"));
        Assert.Equal("Hello policy", document.RootElement.GetProperty("message").GetString());
    }
}
