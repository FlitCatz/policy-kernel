using Xunit;

namespace PolicyKernel.Example.Tests.Unit;

public sealed class AuthorizationTests
{
    [Fact]
    public void Acceptance_token_is_authorized()
    {
        Assert.True(GreetingAuthorization.IsAuthorized("Bearer acceptance-token"));
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("Bearer rejected-token")]
    public void Missing_or_invalid_credentials_are_rejected(string? authorization)
    {
        Assert.False(GreetingAuthorization.IsAuthorized(authorization));
    }
}
