using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Mvc.Testing;
using Xunit;

namespace PolicyKernel.Example.Tests.Integration;

public sealed class GreetingEndpointTests : IClassFixture<WebApplicationFactory<Program>>
{
    private readonly HttpClient _client;

    public GreetingEndpointTests(WebApplicationFactory<Program> application)
    {
        _client = application.CreateClient();
    }

    [Fact]
    public async Task Request_without_credentials_is_unauthorized()
    {
        using var response = await _client.GetAsync("/api/greeting");

        Assert.Equal(HttpStatusCode.Unauthorized, response.StatusCode);
    }

    [Fact]
    public async Task Request_with_acceptance_token_returns_the_greeting_contract()
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, "/api/greeting");
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", "acceptance-token");

        using var response = await _client.SendAsync(request);
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var payload = await response.Content.ReadFromJsonAsync<GreetingPayload>();

        Assert.Equal("Hello policy", payload?.Message);
    }

    private sealed record GreetingPayload(string Message);
}
