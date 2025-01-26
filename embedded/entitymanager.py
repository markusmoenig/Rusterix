import pickle
from enum import Enum

class EntityType(Enum):
    NPC = 1
    PLAYER = 2

class EntityManager:
    def __init__(self, id):
        self.id = id
        self.entities = {}  # Dictionary to store entities by integer ID
        self.next_id = 0    # Counter for generating unique IDs

    def add_entity(self, entity):
        """Adds an entity and assigns it a unique integer ID."""

        if not isinstance(entity, Entity):
            raise TypeError("Only Entity instances can be added.")

        entity_id = self.next_id
        self.next_id += 1

        entity.id = entity_id
        entity.manager_id = self.id

        if entity.type == EntityType.PLAYER:
            register_player(self.id, entity_id)

        self.entities[entity_id] = entity
        return entity_id

    def get_entity(self, entity_id):
        """Retrieves an entity by its ID."""
        if entity_id in self.entities:
            return self.entities[entity_id]
        else:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")

    def broadcast(self, event, value):
        """Broadcasts an event to all entities."""
        for entity_id, entity in self.entities.items():
            entity.event(event, value)

    def event(self, entity_id, event, value):
        """Event"""
        self.entities[entity_id].event(event, value)

    def user_event(self, entity_id, event, value):
        """User based event"""
        self.entities[entity_id].user_event(event, value)

    def delete_entity(self, entity_id):
        """Deletes an entity by its ID."""
        if entity_id in self.entities:
            del self.entities[entity_id]
        else:
            raise KeyError(f"Entity with ID {entity_id} does not exist.")

    def get_all_entities(self):
        """Returns all entities and their attributes."""
        return {eid: entity.get_all_attributes() for eid, entity in self.entities.items()}

    def serialize(self):
        """Serializes the EntityManager to a pickle string."""
        return pickle.dumps(self)

    @staticmethod
    def deserialize(data):
        """Deserializes an EntityManager from a pickle string."""
        return pickle.loads(data)

    def debug(self):
        """Prints all entities and their attributes."""
        print("Entities:")
        for entity_id, entity in self.entities.items():
            print(f" - ID {entity_id}: ")
