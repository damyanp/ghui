<script lang="ts">
  import { Styles, Navbar, Nav, NavItem, NavbarBrand, Container } from "@sveltestrap/sveltestrap";
  import Pat from "../components/Pat.svelte";
</script>

<Styles />

<Container fluid>
  <Navbar class="border-bottom">
    <NavbarBrand>ghui</NavbarBrand>
    <Nav class="ms-auto" navbar>
      <Pat />
    </Nav>
  </Navbar>
</Container>

<style>
</style>
