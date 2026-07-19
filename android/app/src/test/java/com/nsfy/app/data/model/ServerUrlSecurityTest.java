package com.nsfy.app.data.model;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;

import org.junit.Test;

public final class ServerUrlSecurityTest {
    @Test
    public void permitsHttpsAndLoopbackHttp() {
        assertEquals("https://push.example.com", ModelsKt.normalizeServerUrl("https://push.example.com/"));
        assertEquals("http://localhost:8080", ModelsKt.normalizeServerUrl("http://localhost:8080"));
        assertEquals("http://127.0.0.1:8080", ModelsKt.normalizeServerUrl("http://127.0.0.1:8080"));
        assertEquals("http://[::1]:8080", ModelsKt.normalizeServerUrl("http://[::1]:8080"));
    }

    @Test
    public void rejectsRemoteCleartextAndUrlCredentials() {
        assertThrows(
                IllegalArgumentException.class,
                () -> ModelsKt.normalizeServerUrl("http://192.168.1.20:8080"));
        assertThrows(
                IllegalArgumentException.class,
                () -> ModelsKt.normalizeServerUrl("https://token@example.com"));
    }
}
